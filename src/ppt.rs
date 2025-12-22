//! PPT Report Generator Module
//! Generates PowerPoint presentations with chart images (4 images per slide).
//!
//! Uses direct ZIP/XML generation to properly embed images since the ppt-rs
//! high-level API doesn't fully support image embedding yet.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use zip::write::FileOptions;
use zip::ZipWriter;

/// PPT generator for creating chart reports
pub struct PptGenerator;

/// EMU (English Metric Units) conversion: 914400 EMU = 1 inch
const EMU_PER_INCH: i64 = 914400;
/// Standard 16:9 slide dimensions (in EMU)
const SLIDE_WIDTH: i64 = 9144000; // 10 inches
const SLIDE_HEIGHT: i64 = 6858000; // 7.5 inches

impl PptGenerator {
    /// Generate PPT with images from in-memory byte arrays (4 images per slide, 2x2 grid)
    ///
    /// This version takes PNG images as raw bytes, avoiding disk I/O for temp files.
    pub fn generate_ppt_from_bytes(
        image_data: &[Vec<u8>],
        output_path: &Path,
        title: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::create(output_path)?;
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::default();

        // Calculate layout (2x2 grid with margins)
        let margin = EMU_PER_INCH / 2;
        let gap = EMU_PER_INCH / 4;
        let content_width = SLIDE_WIDTH - 2 * margin;
        let content_height = SLIDE_HEIGHT - 2 * margin;
        let img_width = (content_width - gap) / 2;
        let img_height = (content_height - gap) / 2;

        let positions: [(i64, i64); 4] = [
            (margin, margin),
            (margin + img_width + gap, margin),
            (margin, margin + img_height + gap),
            (margin + img_width + gap, margin + img_height + gap),
        ];

        let slides: Vec<_> = image_data.chunks(4).collect();
        let slide_count = slides.len();

        // 1. [Content_Types].xml
        zip.start_file("[Content_Types].xml", options)?;
        zip.write_all(Self::content_types_xml(slide_count, image_data.len()).as_bytes())?;

        // 2. _rels/.rels
        zip.start_file("_rels/.rels", options)?;
        zip.write_all(Self::rels_xml().as_bytes())?;

        // 3. ppt/_rels/presentation.xml.rels
        zip.start_file("ppt/_rels/presentation.xml.rels", options)?;
        zip.write_all(Self::presentation_rels_xml(slide_count).as_bytes())?;

        // 4. ppt/presentation.xml
        zip.start_file("ppt/presentation.xml", options)?;
        zip.write_all(Self::presentation_xml(title, slide_count).as_bytes())?;

        // 5. Slides and their relationships
        let mut global_img_idx = 0;
        for (slide_idx, chunk) in slides.iter().enumerate() {
            let slide_num = slide_idx + 1;
            let img_start = global_img_idx;
            let img_end = global_img_idx + chunk.len();
            global_img_idx = img_end;

            zip.start_file(
                format!("ppt/slides/_rels/slide{}.xml.rels", slide_num),
                options,
            )?;
            let image_ids: Vec<usize> = (img_start..img_end).map(|i| i + 1).collect();
            zip.write_all(Self::slide_rels_xml(&image_ids).as_bytes())?;

            zip.start_file(format!("ppt/slides/slide{}.xml", slide_num), options)?;
            let slide_positions: Vec<(i64, i64, i64, i64)> = (0..chunk.len())
                .map(|i| (positions[i].0, positions[i].1, img_width, img_height))
                .collect();
            zip.write_all(Self::slide_xml(slide_num, &image_ids, &slide_positions).as_bytes())?;
        }

        // 6. Slide layouts
        zip.start_file("ppt/slideLayouts/slideLayout1.xml", options)?;
        zip.write_all(Self::slide_layout_xml().as_bytes())?;
        zip.start_file("ppt/slideLayouts/_rels/slideLayout1.xml.rels", options)?;
        zip.write_all(Self::layout_rels_xml().as_bytes())?;

        // 7. Slide master
        zip.start_file("ppt/slideMasters/slideMaster1.xml", options)?;
        zip.write_all(Self::slide_master_xml().as_bytes())?;
        zip.start_file("ppt/slideMasters/_rels/slideMaster1.xml.rels", options)?;
        zip.write_all(Self::master_rels_xml().as_bytes())?;

        // 8. Theme
        zip.start_file("ppt/theme/theme1.xml", options)?;
        zip.write_all(Self::theme_xml().as_bytes())?;

        // 9. docProps
        zip.start_file("docProps/core.xml", options)?;
        zip.write_all(Self::core_props_xml(title).as_bytes())?;
        zip.start_file("docProps/app.xml", options)?;
        zip.write_all(Self::app_props_xml(slide_count).as_bytes())?;

        // 10. Embed images directly from byte arrays
        for (idx, img_bytes) in image_data.iter().enumerate() {
            zip.start_file(format!("ppt/media/image{}.png", idx + 1), options)?;
            zip.write_all(img_bytes)?;
        }

        zip.finish()?;

        println!(
            "PPT generated: {} ({} slides, {} images)",
            output_path.display(),
            slide_count,
            image_data.len()
        );
        Ok(())
    }

    /// Generate PPT with 4 images per slide (2x2 grid layout) - file-based version
    #[allow(dead_code)]
    pub fn generate_ppt(
        image_paths: &[PathBuf],
        output_path: &Path,
        title: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::create(output_path)?;
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::default();

        // Calculate layout (2x2 grid with margins)
        let margin = EMU_PER_INCH / 2; // 0.5 inch
        let gap = EMU_PER_INCH / 4; // 0.25 inch
        let content_width = SLIDE_WIDTH - 2 * margin;
        let content_height = SLIDE_HEIGHT - 2 * margin;
        let img_width = (content_width - gap) / 2;
        let img_height = (content_height - gap) / 2;

        // 2x2 grid positions
        let positions: [(i64, i64); 4] = [
            (margin, margin),                                      // Top-left
            (margin + img_width + gap, margin),                    // Top-right
            (margin, margin + img_height + gap),                   // Bottom-left
            (margin + img_width + gap, margin + img_height + gap), // Bottom-right
        ];

        // Group images into slides (4 per slide)
        let slides: Vec<_> = image_paths.chunks(4).collect();
        let slide_count = slides.len();

        // 1. [Content_Types].xml
        zip.start_file("[Content_Types].xml", options)?;
        zip.write_all(Self::content_types_xml(slide_count, image_paths.len()).as_bytes())?;

        // 2. _rels/.rels
        zip.start_file("_rels/.rels", options)?;
        zip.write_all(Self::rels_xml().as_bytes())?;

        // 3. ppt/_rels/presentation.xml.rels
        zip.start_file("ppt/_rels/presentation.xml.rels", options)?;
        zip.write_all(Self::presentation_rels_xml(slide_count).as_bytes())?;

        // 4. ppt/presentation.xml
        zip.start_file("ppt/presentation.xml", options)?;
        zip.write_all(Self::presentation_xml(title, slide_count).as_bytes())?;

        // 5. Slides and their relationships
        let mut global_img_idx = 0;
        for (slide_idx, chunk) in slides.iter().enumerate() {
            let slide_num = slide_idx + 1;

            // Calculate image indices for this slide
            let img_start = global_img_idx;
            let img_end = global_img_idx + chunk.len();
            global_img_idx = img_end;

            // Slide relationships
            zip.start_file(
                format!("ppt/slides/_rels/slide{}.xml.rels", slide_num),
                options,
            )?;
            let image_ids: Vec<usize> = (img_start..img_end).map(|i| i + 1).collect();
            zip.write_all(Self::slide_rels_xml(&image_ids).as_bytes())?;

            // Slide content
            zip.start_file(format!("ppt/slides/slide{}.xml", slide_num), options)?;
            let slide_positions: Vec<(i64, i64, i64, i64)> = (0..chunk.len())
                .map(|i| (positions[i].0, positions[i].1, img_width, img_height))
                .collect();
            zip.write_all(Self::slide_xml(slide_num, &image_ids, &slide_positions).as_bytes())?;
        }

        // 6. Slide layouts
        zip.start_file("ppt/slideLayouts/slideLayout1.xml", options)?;
        zip.write_all(Self::slide_layout_xml().as_bytes())?;

        zip.start_file("ppt/slideLayouts/_rels/slideLayout1.xml.rels", options)?;
        zip.write_all(Self::layout_rels_xml().as_bytes())?;

        // 7. Slide master
        zip.start_file("ppt/slideMasters/slideMaster1.xml", options)?;
        zip.write_all(Self::slide_master_xml().as_bytes())?;

        zip.start_file("ppt/slideMasters/_rels/slideMaster1.xml.rels", options)?;
        zip.write_all(Self::master_rels_xml().as_bytes())?;

        // 8. Theme
        zip.start_file("ppt/theme/theme1.xml", options)?;
        zip.write_all(Self::theme_xml().as_bytes())?;

        // 9. docProps
        zip.start_file("docProps/core.xml", options)?;
        zip.write_all(Self::core_props_xml(title).as_bytes())?;

        zip.start_file("docProps/app.xml", options)?;
        zip.write_all(Self::app_props_xml(slide_count).as_bytes())?;

        // 10. EMBED IMAGES (this is the key part!)
        for (idx, img_path) in image_paths.iter().enumerate() {
            let img_data = fs::read(img_path)?;
            let ext = img_path
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            zip.start_file(format!("ppt/media/image{}.{}", idx + 1, ext), options)?;
            zip.write_all(&img_data)?;
        }

        zip.finish()?;

        println!(
            "PPT generated: {} ({} slides, {} images)",
            output_path.display(),
            slide_count,
            image_paths.len()
        );
        Ok(())
    }

    fn content_types_xml(slide_count: usize, _image_count: usize) -> String {
        let mut xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Default Extension="png" ContentType="image/png"/>
<Default Extension="jpg" ContentType="image/jpeg"/>
<Default Extension="jpeg" ContentType="image/jpeg"/>
<Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
<Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/>
<Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>
<Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
<Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
<Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/>
"#.to_string();

        for i in 1..=slide_count {
            xml.push_str(&format!(
                r#"<Override PartName="/ppt/slides/slide{}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>"#,
                i
            ));
            xml.push('\n');
        }
        xml.push_str("</Types>");
        xml
    }

    fn rels_xml() -> &'static str {
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/>
</Relationships>"#
    }

    fn presentation_rels_xml(slide_count: usize) -> String {
        let mut xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
"#.to_string();

        for i in 1..=slide_count {
            xml.push_str(&format!(
                r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{}.xml"/>"#,
                i + 2, i
            ));
            xml.push('\n');
        }
        xml.push_str("</Relationships>");
        xml
    }

    fn presentation_xml(_title: &str, slide_count: usize) -> String {
        let mut slide_ids = String::new();
        for i in 1..=slide_count {
            slide_ids.push_str(&format!(
                r#"<p:sldId id="{}" r:id="rId{}"/>"#,
                255 + i,
                i + 2
            ));
        }

        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" saveSubsetFonts="1">
<p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId1"/></p:sldMasterIdLst>
<p:sldIdLst>{}</p:sldIdLst>
<p:sldSz cx="{}" cy="{}" type="screen16x9"/>
<p:notesSz cx="{}" cy="{}"/>
</p:presentation>"#,
            slide_ids, SLIDE_WIDTH, SLIDE_HEIGHT, SLIDE_HEIGHT, SLIDE_WIDTH
        )
    }

    fn slide_rels_xml(image_ids: &[usize]) -> String {
        let mut xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
"#.to_string();

        for (idx, img_id) in image_ids.iter().enumerate() {
            xml.push_str(&format!(
                r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image{}.png"/>"#,
                idx + 2, img_id
            ));
            xml.push('\n');
        }
        xml.push_str("</Relationships>");
        xml
    }

    fn slide_xml(
        _slide_num: usize,
        image_ids: &[usize],
        positions: &[(i64, i64, i64, i64)],
    ) -> String {
        let mut shapes = String::new();

        for (idx, ((x, y, w, h), _img_id)) in positions.iter().zip(image_ids.iter()).enumerate() {
            let shape_id = idx + 2;
            let r_id = idx + 2;
            shapes.push_str(&format!(
                r#"
<p:pic>
<p:nvPicPr>
<p:cNvPr id="{}" name="Picture {}"/>
<p:cNvPicPr><a:picLocks noChangeAspect="1"/></p:cNvPicPr>
<p:nvPr/>
</p:nvPicPr>
<p:blipFill>
<a:blip r:embed="rId{}"/>
<a:stretch><a:fillRect/></a:stretch>
</p:blipFill>
<p:spPr>
<a:xfrm><a:off x="{}" y="{}"/><a:ext cx="{}" cy="{}"/></a:xfrm>
<a:prstGeom prst="rect"><a:avLst/></a:prstGeom>
</p:spPr>
</p:pic>"#,
                shape_id, shape_id, r_id, x, y, w, h
            ));
        }

        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld>
<p:spTree>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
<p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>
{}
</p:spTree>
</p:cSld>
<p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>
</p:sld>"#,
            shapes
        )
    }

    fn slide_layout_xml() -> &'static str {
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" type="blank" preserve="1">
<p:cSld name="Blank"><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr></p:spTree></p:cSld>
<p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>
</p:sldLayout>"#
    }

    fn layout_rels_xml() -> &'static str {
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/>
</Relationships>"#
    }

    fn slide_master_xml() -> &'static str {
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:bg><p:bgRef idx="1001"><a:schemeClr val="bg1"/></p:bgRef></p:bg><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr></p:spTree></p:cSld>
<p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
<p:sldLayoutIdLst><p:sldLayoutId id="2147483649" r:id="rId1"/></p:sldLayoutIdLst>
</p:sldMaster>"#
    }

    fn master_rels_xml() -> &'static str {
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="../theme/theme1.xml"/>
</Relationships>"#
    }

    fn theme_xml() -> &'static str {
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Office Theme">
<a:themeElements>
<a:clrScheme name="Office"><a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1><a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="44546A"/></a:dk2><a:lt2><a:srgbClr val="E7E6E6"/></a:lt2><a:accent1><a:srgbClr val="4472C4"/></a:accent1><a:accent2><a:srgbClr val="ED7D31"/></a:accent2><a:accent3><a:srgbClr val="A5A5A5"/></a:accent3><a:accent4><a:srgbClr val="FFC000"/></a:accent4><a:accent5><a:srgbClr val="5B9BD5"/></a:accent5><a:accent6><a:srgbClr val="70AD47"/></a:accent6><a:hlink><a:srgbClr val="0563C1"/></a:hlink><a:folHlink><a:srgbClr val="954F72"/></a:folHlink></a:clrScheme>
<a:fontScheme name="Office"><a:majorFont><a:latin typeface="Calibri Light"/><a:ea typeface=""/><a:cs typeface=""/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/><a:ea typeface=""/><a:cs typeface=""/></a:minorFont></a:fontScheme>
<a:fmtScheme name="Office"><a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:gradFill rotWithShape="1"><a:gsLst><a:gs pos="0"><a:schemeClr val="phClr"><a:tint val="50000"/><a:satMod val="300000"/></a:schemeClr></a:gs><a:gs pos="35000"><a:schemeClr val="phClr"><a:tint val="37000"/><a:satMod val="300000"/></a:schemeClr></a:gs><a:gs pos="100000"><a:schemeClr val="phClr"><a:tint val="15000"/><a:satMod val="350000"/></a:schemeClr></a:gs></a:gsLst><a:lin ang="16200000" scaled="1"/></a:gradFill><a:gradFill rotWithShape="1"><a:gsLst><a:gs pos="0"><a:schemeClr val="phClr"><a:shade val="51000"/><a:satMod val="130000"/></a:schemeClr></a:gs><a:gs pos="80000"><a:schemeClr val="phClr"><a:shade val="93000"/><a:satMod val="130000"/></a:schemeClr></a:gs><a:gs pos="100000"><a:schemeClr val="phClr"><a:shade val="94000"/><a:satMod val="135000"/></a:schemeClr></a:gs></a:gsLst><a:lin ang="16200000" scaled="0"/></a:gradFill></a:fillStyleLst><a:lnStyleLst><a:ln w="6350" cap="flat" cmpd="sng" algn="ctr"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:prstDash val="solid"/><a:miter lim="800000"/></a:ln><a:ln w="12700" cap="flat" cmpd="sng" algn="ctr"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:prstDash val="solid"/><a:miter lim="800000"/></a:ln><a:ln w="19050" cap="flat" cmpd="sng" algn="ctr"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:prstDash val="solid"/><a:miter lim="800000"/></a:ln></a:lnStyleLst><a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst><a:outerShdw blurRad="57150" dist="19050" dir="5400000" algn="ctr" rotWithShape="0"><a:srgbClr val="000000"><a:alpha val="63000"/></a:srgbClr></a:outerShdw></a:effectLst></a:effectStyle></a:effectStyleLst><a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"><a:tint val="95000"/><a:satMod val="170000"/></a:schemeClr></a:solidFill><a:gradFill rotWithShape="1"><a:gsLst><a:gs pos="0"><a:schemeClr val="phClr"><a:tint val="93000"/><a:satMod val="150000"/><a:shade val="98000"/><a:lumMod val="102000"/></a:schemeClr></a:gs><a:gs pos="50000"><a:schemeClr val="phClr"><a:tint val="98000"/><a:satMod val="130000"/><a:shade val="90000"/><a:lumMod val="103000"/></a:schemeClr></a:gs><a:gs pos="100000"><a:schemeClr val="phClr"><a:shade val="63000"/><a:satMod val="120000"/></a:schemeClr></a:gs></a:gsLst><a:lin ang="5400000" scaled="0"/></a:gradFill></a:bgFillStyleLst></a:fmtScheme>
</a:themeElements>
<a:objectDefaults/>
<a:extraClrSchemeLst/>
</a:theme>"#
    }

    fn core_props_xml(title: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:dcmitype="http://purl.org/dc/dcmitype/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
<dc:title>{}</dc:title>
<dc:creator>Chartify Pro</dc:creator>
<cp:lastModifiedBy>Chartify Pro</cp:lastModifiedBy>
<cp:revision>1</cp:revision>
</cp:coreProperties>"#,
            title
        )
    }

    fn app_props_xml(slide_count: usize) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties" xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes">
<TotalTime>0</TotalTime>
<Words>0</Words>
<Application>Chartify Pro</Application>
<PresentationFormat>On-screen Show (16:9)</PresentationFormat>
<Paragraphs>0</Paragraphs>
<Slides>{}</Slides>
<Notes>0</Notes>
<HiddenSlides>0</HiddenSlides>
<MMClips>0</MMClips>
<ScaleCrop>false</ScaleCrop>
<LinksUpToDate>false</LinksUpToDate>
<SharedDoc>false</SharedDoc>
<HyperlinksChanged>false</HyperlinksChanged>
<AppVersion>16.0000</AppVersion>
</Properties>"#,
            slide_count
        )
    }

    /// Export charts to PNG format for PPT embedding
    pub fn export_charts_as_png(
        chart_data: &std::collections::HashMap<String, crate::charts::ChartData>,
        data_type_order: &[String],
        base_path: &Path,
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        use crate::charts::ChartRenderer;

        let tem_path = base_path.join("tem_png");
        fs::create_dir_all(&tem_path)?;

        let mut png_paths = Vec::new();
        let width = 800u32;
        let height = 600u32;

        for data_type in data_type_order {
            if let Some(data) = chart_data.get(data_type) {
                let safe_name: String = data_type
                    .chars()
                    .map(|c| {
                        if c.is_alphanumeric() || c == '_' || c == '-' {
                            c
                        } else {
                            '_'
                        }
                    })
                    .collect();

                let file_path = tem_path.join(format!("{}.png", safe_name));
                ChartRenderer::render_chart_card_png(data, &file_path, width, height)?;
                png_paths.push(file_path);
            }
        }

        Ok(png_paths)
    }
}
