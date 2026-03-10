//! Merge multiple PDF documents (each with one page) into a single PDF.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use lopdf::dictionary;
use lopdf::{Object, ObjectId};

/// Merge multiple PDF byte buffers into one PDF, preserving page order.
/// Each input PDF should typically have one page (from a chunk render).
pub fn merge_pdfs(pdf_bytes_list: &[Vec<u8>]) -> Result<Vec<u8>> {
    if pdf_bytes_list.is_empty() {
        return Ok(Vec::new());
    }
    if pdf_bytes_list.len() == 1 {
        return Ok(pdf_bytes_list[0].clone());
    }

    let mut documents: Vec<lopdf::Document> = pdf_bytes_list
        .iter()
        .map(|bytes| lopdf::Document::load_mem(bytes).context("load PDF chunk"))
        .collect::<Result<Vec<_>>>()?;

    let mut merged = lopdf::Document::with_version("1.5");
    let mut max_id = 1u32;
    let mut all_pages: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let mut all_objects: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let mut first_pages_id: Option<ObjectId> = None;
    let mut first_catalog_id: Option<ObjectId> = None;

    for doc in documents.iter_mut() {
        doc.renumber_objects_with(max_id);
        max_id = doc.max_id + 1;

        for (_, page_id) in doc.get_pages() {
            let page_obj = doc.get_object(page_id).context("get page")?.to_owned();
            all_pages.insert(page_id, page_obj);
        }

        for (id, obj) in doc.objects.iter() {
            match obj.type_name().unwrap_or(b"") {
                b"Pages" if first_pages_id.is_none() => first_pages_id = Some(*id),
                b"Catalog" if first_catalog_id.is_none() => first_catalog_id = Some(*id),
                _ => {}
            }
        }
        all_objects.extend(doc.objects.clone());
    }

    let pages_id = first_pages_id.context("no Pages in PDF")?;
    let catalog_id = first_catalog_id.context("no Catalog in PDF")?;

    let kids: Vec<Object> = all_pages
        .keys()
        .map(|&id| Object::Reference(id))
        .collect();

    let mut pages_dict = dictionary! {
        "Type" => "Pages",
        "Kids" => Object::Array(kids.clone()),
        "Count" => kids.len() as u32,
    };
    if let Some(existing) = all_objects.get(&pages_id).and_then(|o| o.as_dict().ok()) {
        if let Ok(mb) = existing.get(b"MediaBox") {
            pages_dict.set("MediaBox", mb.clone());
        }
        if let Ok(r) = existing.get(b"Resources") {
            pages_dict.set("Resources", r.clone());
        }
    }
    merged.objects.insert(pages_id, Object::Dictionary(pages_dict));

    for (object_id, mut object) in all_pages {
        if let Ok(dict) = object.as_dict_mut() {
            dict.set("Parent", Object::Reference(pages_id));
        }
        merged.objects.insert(object_id, object);
    }

    for (object_id, object) in all_objects.into_iter() {
        match object.type_name().unwrap_or(b"") {
            b"Catalog" if object_id == catalog_id => {
                let mut dict = object.as_dict().context("Catalog not dict")?.clone();
                dict.set("Pages", Object::Reference(pages_id));
                dict.remove(b"Outlines");
                merged.objects.insert(catalog_id, Object::Dictionary(dict));
            }
            b"Catalog" => {}
            b"Pages" => {}
            b"Page" => {}
            b"Outlines" | b"Outline" => {}
            _ => {
                merged.objects.insert(object_id, object);
            }
        }
    }

    merged.trailer.set("Root", catalog_id);
    merged.max_id = merged.objects.len() as u32;
    merged.renumber_objects();

    let mut out = Vec::new();
    merged.save_to(&mut out).context("write merged PDF")?;

    Ok(out)
}
