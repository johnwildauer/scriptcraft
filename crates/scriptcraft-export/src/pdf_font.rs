//! Complete-face PDF font encoding shared by packaged and later approved host
//! faces. Packaged Courier Prime is byte-compared against compile-time bundled
//! programs before the generic CID/GID encoder is called.

use crate::{pdf_document::{PdfDocumentWriter, PdfObjectId}, ExportError, PdfResolvedFaceBytes};
use scriptcraft_model::typography::{
    FaceStyle, LeadingResolverKind, ResolvedFaceMetrics, ResolvedFontSourceKind,
};
use std::collections::{BTreeMap, BTreeSet};
use ttf_parser::{Face, GlyphId};

pub(crate) const MAX_PDF_FONT_RESOURCES: usize = 268;
pub(crate) const MAX_PDF_CIDS_PER_FACE: usize = 4_096;

const COURIER_PRIME_FACES: [&[u8]; 4] = [
    include_bytes!("../../../fonts/CourierPrime/Courier Prime.ttf"),
    include_bytes!("../../../fonts/CourierPrime/Courier Prime Bold.ttf"),
    include_bytes!("../../../fonts/CourierPrime/Courier Prime Italic.ttf"),
    include_bytes!("../../../fonts/CourierPrime/Courier Prime Bold Italic.ttf"),
];

/// All data required to emit one complete composite font. It owns no mutable
/// cache state and preserves original FontFile2 bytes without transformation.
#[derive(Debug)]
pub(crate) struct CompleteFontPlan {
    pub(crate) postscript_name: String,
    pub(crate) cids: BTreeMap<char, u16>,
    pub(crate) missing_scalars: BTreeSet<char>,
    font_bytes: Vec<u8>,
    cid_to_gid: Vec<u8>,
    to_unicode: Vec<u8>,
    widths: Vec<u16>,
    bbox: [i32; 4],
    ascent: i32,
    descent: i32,
    cap_height: i32,
    italic_angle: f32,
    fixed_pitch: bool,
}

/// Validates app-selected packaged provenance, then enters the reusable
/// complete-face encoder. A packaged defect is never downgraded to Base-14.
pub(crate) fn encode_packaged_courier_prime(
    entry: &PdfResolvedFaceBytes,
    visible_scalars: &BTreeSet<char>,
) -> Result<CompleteFontPlan, ExportError> {
    let face = &entry.face;
    let style_index = style_index(face.style);
    if face.source_kind != ResolvedFontSourceKind::Bundled
        || face.family_label != "Courier Prime"
        || face.face_label != courier_style_name(face.style)
        || face.leading_resolver != LeadingResolverKind::Courier
        || entry.face_index != 0
        || entry.bytes.as_ref() != COURIER_PRIME_FACES[style_index]
    {
        return Err(ExportError::RequiredPackagedFontEmbedding);
    }
    let parsed = parse_standalone_face(&entry.bytes, entry.face_index)?;
    let postscript_name = unique_postscript_name(&parsed)?;
    let permissions = parsed
        .permissions()
        .ok_or(ExportError::RequiredPackagedFontEmbedding)?;
    if matches!(permissions, ttf_parser::Permissions::Restricted)
        || !parsed.is_outline_embedding_allowed()
    {
        return Err(ExportError::RequiredPackagedFontEmbedding);
    }
    let tables = parsed.tables();
    if tables.glyf.is_none() || tables.os2.is_none() || tables.post.is_none() {
        return Err(ExportError::RequiredPackagedFontEmbedding);
    }
    encode_complete_face(
        &parsed,
        &postscript_name,
        &face.metrics,
        &entry.bytes,
        visible_scalars,
    )
}

/// Generic seam for WO-004C: eligibility is decided by the caller, while this
/// encoder independently verifies parser/scalar metrics and creates all PDF
/// CID/GID, width, Unicode, descriptor, and complete-program data.
pub(crate) fn encode_complete_face(
    face: &Face<'_>,
    postscript_name: &str,
    metrics: &ResolvedFaceMetrics,
    original_bytes: &[u8],
    visible_scalars: &BTreeSet<char>,
) -> Result<CompleteFontPlan, ExportError> {
    if !valid_postscript_name(postscript_name)
        || u32::from(face.units_per_em()) != metrics.units_per_em
        || i32::from(face.ascender()) != metrics.ascender
        || i32::from(face.descender()) != metrics.descender
        || i32::from(face.line_gap()) != metrics.line_gap
        || visible_scalars.len() > MAX_PDF_CIDS_PER_FACE
    {
        return Err(ExportError::InvalidPdfFontPayload);
    }
    let glyph_zero = face
        .glyph_hor_advance(GlyphId(0))
        .ok_or(ExportError::InvalidPdfFontPayload)?;
    let mut cids = BTreeMap::new();
    let mut missing_scalars = BTreeSet::new();
    let scalar_capacity = visible_scalars
        .len()
        .checked_add(1)
        .ok_or(ExportError::InvalidPdfFontPayload)?;
    let mut glyphs = Vec::with_capacity(scalar_capacity);
    let mut widths = Vec::with_capacity(scalar_capacity);
    glyphs.push(0u16);
    widths.push(scale_width(glyph_zero, metrics.units_per_em)?);

    for (index, scalar) in visible_scalars.iter().copied().enumerate() {
        let cid = u16::try_from(index.checked_add(1).ok_or(ExportError::InvalidPdfFontPayload)?)
            .map_err(|_| ExportError::InvalidPdfFontPayload)?;
        let glyph = face.glyph_index(scalar);
        let expected = metrics
            .advance_for(scalar)
            .ok_or(ExportError::InvalidPdfFontPayload)?;
        let is_missing = glyph.map_or(true, |glyph_id| glyph_id.0 == 0);
        if expected.missing_glyph != is_missing {
            return Err(ExportError::InvalidPdfFontPayload);
        }
        let glyph_id = glyph.unwrap_or(GlyphId(0));
        let advance = face
            .glyph_hor_advance(glyph_id)
            .ok_or(ExportError::InvalidPdfFontPayload)?;
        if u32::from(advance) != expected.advance_units {
            return Err(ExportError::InvalidPdfFontPayload);
        }
        if is_missing {
            missing_scalars.insert(scalar);
        }
        cids.insert(scalar, cid);
        glyphs.push(glyph_id.0);
        widths.push(scale_width(advance, metrics.units_per_em)?);
    }

    let cid_to_gid = encode_cid_to_gid(&glyphs)?;
    let to_unicode = encode_to_unicode(&cids)?;
    let tables = face.tables();
    let head = &tables.head;
    let post = tables
        .post
        .as_ref()
        .ok_or(ExportError::InvalidPdfFontPayload)?;
    let os2 = tables
        .os2
        .as_ref()
        .ok_or(ExportError::InvalidPdfFontPayload)?;
    let cap_height = os2
        .capital_height()
        .map(i32::from)
        .unwrap_or_else(|| i32::from(face.ascender()));

    Ok(CompleteFontPlan {
        postscript_name: postscript_name.to_owned(),
        cids,
        missing_scalars,
        font_bytes: original_bytes.to_vec(),
        cid_to_gid,
        to_unicode,
        widths,
        bbox: [
            i32::from(head.global_bbox.x_min),
            i32::from(head.global_bbox.y_min),
            i32::from(head.global_bbox.x_max),
            i32::from(head.global_bbox.y_max),
        ],
        ascent: i32::from(face.ascender()),
        descent: i32::from(face.descender()),
        cap_height,
        italic_angle: if post.italic_angle.is_finite() {
            post.italic_angle
        } else {
            return Err(ExportError::InvalidPdfFontPayload);
        },
        fixed_pitch: post.is_monospaced,
    })
}

/// Writes the six-object composite-font graph in the caller's stable
/// allocation order and returns its Type0 resource object ID.
pub(crate) fn write_font_objects(
    writer: &mut PdfDocumentWriter,
    plan: &CompleteFontPlan,
) -> Result<PdfObjectId, ExportError> {
    let font_file_id = writer.reserve_object()?;
    let descriptor_id = writer.reserve_object()?;
    let cid_map_id = writer.reserve_object()?;
    let descendant_id = writer.reserve_object()?;
    let unicode_id = writer.reserve_object()?;
    let type0_id = writer.reserve_object()?;

    let font_length = u64::try_from(plan.font_bytes.len())
        .map_err(|_| ExportError::OutputResourceLimitExceeded)?;
    let font_file_dict = format!("/Length1 {font_length}").into_bytes();
    writer.write_stream(font_file_id, font_file_dict, plan.font_bytes.clone(), Vec::new())?;

    let mut flags = 32i32; // Nonsymbolic; the CMap maps Unicode explicitly.
    if plan.fixed_pitch {
        flags |= 1;
    }
    if plan.italic_angle != 0.0 {
        flags |= 64;
    }
    let descriptor = format!(
        "<< /Type /FontDescriptor /FontName /{} /Flags {} /FontBBox [{} {} {} {}] /ItalicAngle {:.3} /Ascent {} /Descent {} /CapHeight {} /StemV 80 /FontFile2 {} 0 R >>",
        plan.postscript_name,
        flags,
        plan.bbox[0],
        plan.bbox[1],
        plan.bbox[2],
        plan.bbox[3],
        plan.italic_angle,
        plan.ascent,
        plan.descent,
        plan.cap_height,
        font_file_id.0,
    )
    .into_bytes();
    writer.write_object(descriptor_id, descriptor, vec![font_file_id])?;

    writer.write_stream(
        cid_map_id,
        Vec::new(),
        plan.cid_to_gid.clone(),
        Vec::new(),
    )?;
    let widths = serialize_widths(&plan.widths)?;
    let descendant = format!(
        "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /{} /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor {} 0 R /DW 1000 /W {} /CIDToGIDMap {} 0 R >>",
        plan.postscript_name,
        descriptor_id.0,
        widths,
        cid_map_id.0,
    )
    .into_bytes();
    writer.write_object(
        descendant_id,
        descendant,
        vec![descriptor_id, cid_map_id],
    )?;

    writer.write_stream(
        unicode_id,
        Vec::new(),
        plan.to_unicode.clone(),
        Vec::new(),
    )?;
    let type0 = format!(
        "<< /Type /Font /Subtype /Type0 /BaseFont /{} /Encoding /Identity-H /DescendantFonts [{} 0 R] /ToUnicode {} 0 R >>",
        plan.postscript_name,
        descendant_id.0,
        unicode_id.0,
    )
    .into_bytes();
    writer.write_object(type0_id, type0, vec![descendant_id, unicode_id])?;
    Ok(type0_id)
}

fn parse_standalone_face(bytes: &[u8], face_index: u32) -> Result<Face<'_>, ExportError> {
    if face_index != 0 || bytes.get(..4) != Some(&[0, 1, 0, 0]) || bytes.starts_with(b"ttcf") {
        return Err(ExportError::RequiredPackagedFontEmbedding);
    }
    let face = Face::parse(bytes, face_index)
        .map_err(|_| ExportError::RequiredPackagedFontEmbedding)?;
    if face.is_variable() {
        return Err(ExportError::RequiredPackagedFontEmbedding);
    }
    Ok(face)
}

fn unique_postscript_name(face: &Face<'_>) -> Result<String, ExportError> {
    let names = face
        .names()
        .into_iter()
        .filter(|name| name.name_id == 6)
        .filter_map(|name| name.to_string())
        .collect::<BTreeSet<_>>();
    if names.len() != 1 {
        return Err(ExportError::RequiredPackagedFontEmbedding);
    }
    let name = names
        .into_iter()
        .next()
        .ok_or(ExportError::RequiredPackagedFontEmbedding)?;
    if !valid_postscript_name(&name) {
        return Err(ExportError::RequiredPackagedFontEmbedding);
    }
    Ok(name.to_owned())
}

fn valid_postscript_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 127
        && name.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'+')
        })
}

fn style_index(style: FaceStyle) -> usize {
    match style {
        FaceStyle::Regular => 0,
        FaceStyle::Bold => 1,
        FaceStyle::Italic => 2,
        FaceStyle::BoldItalic => 3,
    }
}

fn courier_style_name(style: FaceStyle) -> &'static str {
    match style {
        FaceStyle::Regular => "Courier Prime Regular",
        FaceStyle::Bold => "Courier Prime Bold",
        FaceStyle::Italic => "Courier Prime Italic",
        FaceStyle::BoldItalic => "Courier Prime Bold Italic",
    }
}

fn scale_width(advance: u16, units_per_em: u32) -> Result<u16, ExportError> {
    if units_per_em == 0 {
        return Err(ExportError::InvalidPdfFontPayload);
    }
    let numerator = u64::from(advance)
        .checked_mul(1000)
        .and_then(|value| value.checked_add(u64::from(units_per_em) / 2))
        .ok_or(ExportError::InvalidPdfFontPayload)?;
    let width = numerator / u64::from(units_per_em);
    u16::try_from(width).map_err(|_| ExportError::InvalidPdfFontPayload)
}

fn encode_cid_to_gid(glyphs: &[u16]) -> Result<Vec<u8>, ExportError> {
    let capacity = glyphs
        .len()
        .checked_mul(2)
        .ok_or(ExportError::InvalidPdfFontPayload)?;
    let mut bytes = Vec::with_capacity(capacity);
    for glyph in glyphs {
        bytes.extend_from_slice(&glyph.to_be_bytes());
    }
    Ok(bytes)
}

fn encode_to_unicode(cids: &BTreeMap<char, u16>) -> Result<Vec<u8>, ExportError> {
    let mut output = b"/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /ScriptCraft-ToUnicode def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n".to_vec();
    for block in cids.iter().collect::<Vec<_>>().chunks(100) {
        output.extend_from_slice(format!("{} beginbfchar\n", block.len()).as_bytes());
        for (scalar, cid) in block {
            let mut utf16 = [0u16; 2];
            let encoded = scalar.encode_utf16(&mut utf16);
            output.extend_from_slice(format!("<{cid:04X}> <").as_bytes());
            for unit in encoded {
                output.extend_from_slice(format!("{unit:04X}").as_bytes());
            }
            output.extend_from_slice(b">\n");
        }
        output.extend_from_slice(b"endbfchar\n");
    }
    output.extend_from_slice(b"endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
    Ok(output)
}

fn serialize_widths(widths: &[u16]) -> Result<String, ExportError> {
    if widths.is_empty() {
        return Ok("[]".to_owned());
    }
    let mut serialized = String::from("[0 [");
    for (index, width) in widths.iter().enumerate() {
        if index > 0 {
            serialized.push(' ');
        }
        serialized.push_str(&width.to_string());
    }
    serialized.push_str("]] ");
    Ok(serialized)
}
