use crate::archive::resolve_relative_path;
use crate::metadata::{GuideItem, ManifestItem, Metadata, PageProgressionDirection, SpineItem};
use ahash::AHashMap;
use roxmltree::Document;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OpfPackage {
    pub version: String,
    pub opf_path: String,
    pub opf_dir: String,
    pub metadata: Metadata,
    pub manifest: AHashMap<String, ManifestItem>,
    pub spine: Vec<SpineItem>,
    pub guide: Vec<GuideItem>,
    pub toc_item_id: Option<String>,
    pub nav_item_id: Option<String>,
}

/// Parse `META-INF/container.xml` to get the rootfile path to `.opf`.
pub fn parse_container_xml(xml_content: &str) -> Result<String, String> {
    let doc = Document::parse(xml_content)
        .map_err(|e| format!("XML parse error in container.xml: {}", e))?;

    for node in doc.descendants() {
        if node.has_tag_name("rootfile") {
            if let Some(full_path) = node.attribute("full-path") {
                return Ok(full_path.to_string());
            }
        }
    }

    Err("Could not find rootfile element in META-INF/container.xml".to_string())
}

/// Parse the `.opf` package file into `OpfPackage`.
pub fn parse_opf(xml_content: &str, opf_path: &str) -> Result<OpfPackage, String> {
    let repaired_xml;
    let effective_xml = match Document::parse(xml_content) {
        Ok(_) => xml_content,
        Err(_) => {
            repaired_xml = crate::dom::sanitize_and_repair_xml(xml_content);
            &repaired_xml
        }
    };
    let doc = Document::parse(effective_xml)
        .map_err(|e| format!("XML parse error in OPF file: {}", e))?;
    let root = doc.root_element();

    if root.tag_name().name() != "package" {
        return Err("OPF root element must be <package>".to_string());
    }

    let version = root.attribute("version").unwrap_or("2.0").to_string();

    // Determine OPF directory
    let opf_dir = if let Some(last_slash) = opf_path.rfind('/') {
        &opf_path[..last_slash]
    } else {
        ""
    };

    let mut metadata = Metadata::default();
    let mut manifest = AHashMap::new();
    let mut spine = Vec::new();
    let mut guide = Vec::new();
    let mut toc_item_id = None;
    let mut nav_item_id = None;

    // Parse child elements of <package>
    for child in root.children() {
        if !child.is_element() {
            continue;
        }

        match child.tag_name().name() {
            "metadata" => {
                parse_metadata_node(&child, &mut metadata)?;
            }
            "manifest" => {
                for item_node in child.children() {
                    if item_node.has_tag_name("item") {
                        if let (Some(id), Some(href), Some(media_type)) = (
                            item_node.attribute("id"),
                            item_node.attribute("href"),
                            item_node.attribute("media-type"),
                        ) {
                            let properties_str = item_node.attribute("properties").unwrap_or("");
                            let properties: Vec<String> = properties_str
                                .split_whitespace()
                                .map(|s| s.to_string())
                                .collect();

                            let full_path = resolve_relative_path(opf_dir, href);

                            if properties.contains(&"nav".to_string()) {
                                nav_item_id = Some(id.to_string());
                            }
                            if properties.contains(&"cover-image".to_string()) {
                                metadata.cover_id = Some(id.to_string());
                                metadata.cover_href = Some(full_path.clone());
                            }

                            let item = ManifestItem {
                                id: id.to_string(),
                                href: href.to_string(),
                                full_path,
                                media_type: media_type.to_string(),
                                properties,
                                fallback: item_node.attribute("fallback").map(|s| s.to_string()),
                                media_overlay: item_node
                                    .attribute("media-overlay")
                                    .map(|s| s.to_string()),
                            };
                            manifest.insert(id.to_string(), item);
                        }
                    }
                }
            }
            "spine" => {
                if let Some(toc_id) = child.attribute("toc") {
                    toc_item_id = Some(toc_id.to_string());
                }

                if let Some(dir_str) = child.attribute("page-progression-direction") {
                    metadata.direction = match dir_str {
                        "rtl" => PageProgressionDirection::Rtl,
                        "ltr" => PageProgressionDirection::Ltr,
                        _ => PageProgressionDirection::Default,
                    };
                }

                let mut spine_idx = 0;
                for itemref in child.children() {
                    if itemref.has_tag_name("itemref") {
                        if let Some(idref) = itemref.attribute("idref") {
                            let linear = itemref.attribute("linear").unwrap_or("yes") != "no";
                            let properties_str = itemref.attribute("properties").unwrap_or("");
                            let properties: Vec<String> = properties_str
                                .split_whitespace()
                                .map(|s| s.to_string())
                                .collect();

                            if let Some(manifest_item) = manifest.get(idref) {
                                spine.push(SpineItem {
                                    idref: idref.to_string(),
                                    linear,
                                    properties,
                                    index: spine_idx,
                                    href: manifest_item.full_path.clone(),
                                    media_type: manifest_item.media_type.clone(),
                                });
                                spine_idx += 1;
                            }
                        }
                    }
                }
            }
            "guide" => {
                for reference in child.children() {
                    if reference.has_tag_name("reference") {
                        if let (Some(type_), Some(href)) =
                            (reference.attribute("type"), reference.attribute("href"))
                        {
                            let title = reference.attribute("title").unwrap_or("").to_string();
                            let full_path = resolve_relative_path(opf_dir, href);

                            if type_ == "cover" && metadata.cover_href.is_none() {
                                metadata.cover_href = Some(full_path.clone());
                            }

                            guide.push(GuideItem {
                                type_: type_.to_string(),
                                title,
                                href: href.to_string(),
                                full_path,
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Secondary fallback for cover image if meta name="cover" was used
    if metadata.cover_href.is_none() {
        if let Some(cover_id) = &metadata.cover_id {
            if let Some(item) = manifest.get(cover_id) {
                metadata.cover_href = Some(item.full_path.clone());
            }
        }
    }

    Ok(OpfPackage {
        version,
        opf_path: opf_path.to_string(),
        opf_dir: opf_dir.to_string(),
        metadata,
        manifest,
        spine,
        guide,
        toc_item_id,
        nav_item_id,
    })
}

fn parse_metadata_node(node: &roxmltree::Node, metadata: &mut Metadata) -> Result<(), String> {
    for child in node.children() {
        if !child.is_element() {
            continue;
        }

        let name = child.tag_name().name();
        let text = child.text().unwrap_or("").trim().to_string();

        match name {
            "title" => {
                if !text.is_empty() {
                    if metadata.title.is_empty() {
                        metadata.title = text;
                    } else {
                        metadata.title = format!("{}: {}", metadata.title, text);
                    }
                }
            }
            "creator" => {
                if !text.is_empty() {
                    metadata.creators.push(text);
                }
            }
            "publisher" => {
                if !text.is_empty() {
                    metadata.publishers.push(text);
                }
            }
            "language" => {
                if !text.is_empty() {
                    metadata.languages.push(text);
                }
            }
            "rights" => metadata.rights = Some(text),
            "description" => metadata.description = Some(text),
            "identifier" => metadata.identifier = Some(text),
            "date" => metadata.pub_date = Some(text),
            "subject" => {
                if !text.is_empty() {
                    metadata.subjects.push(text);
                }
            }
            "meta" => {
                if let Some(name_attr) = child.attribute("name") {
                    if name_attr == "cover" {
                        if let Some(content) = child.attribute("content") {
                            metadata.cover_id = Some(content.to_string());
                        }
                    }
                }
                if let Some(property) = child.attribute("property") {
                    metadata
                        .meta_properties
                        .insert(property.to_string(), text.clone());
                    if property == "dcterms:modified" {
                        metadata.modified_date = Some(text.clone());
                    }
                    parse_a11y_property(property, &text, &mut metadata.accessibility);
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn parse_a11y_property(
    property: &str,
    text: &str,
    a11y: &mut crate::metadata::AccessibilityMetadata,
) {
    let prop = property.trim();
    if prop == "schema:accessMode" || prop == "accessMode" {
        if !text.is_empty() && !a11y.access_modes.contains(&text.to_string()) {
            a11y.access_modes.push(text.to_string());
        }
    } else if prop == "schema:accessModeSufficient" || prop == "accessModeSufficient" {
        let modes: Vec<String> = text.split(',').map(|s| s.trim().to_string()).collect();
        if !modes.is_empty() {
            a11y.access_modes_sufficient.push(modes);
        }
    } else if prop == "schema:accessibilityFeature" || prop == "accessibilityFeature" {
        if !text.is_empty() && !a11y.accessibility_features.contains(&text.to_string()) {
            a11y.accessibility_features.push(text.to_string());
        }
    } else if prop == "schema:accessibilityHazard" || prop == "accessibilityHazard" {
        if !text.is_empty() && !a11y.accessibility_hazards.contains(&text.to_string()) {
            a11y.accessibility_hazards.push(text.to_string());
        }
    } else if prop == "schema:accessibilitySummary" || prop == "accessibilitySummary" {
        a11y.accessibility_summary = Some(text.to_string());
    } else if prop == "a11y:certifiedBy" || prop == "certifiedBy" {
        a11y.certified_by = Some(text.to_string());
    } else if prop == "a11y:certifierCredential" || prop == "certifierCredential" {
        a11y.certifier_credential = Some(text.to_string());
    } else if prop == "a11y:certifierReport" || prop == "certifierReport" {
        a11y.certifier_report = Some(text.to_string());
    }

    if !a11y.access_modes.is_empty() || !a11y.accessibility_features.is_empty() {
        a11y.is_accessible = true;
    }
}
