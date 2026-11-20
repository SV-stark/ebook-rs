//! Pure-Rust multi-format eBook, archive, office, and markup codecs (Input & Output).

pub mod kindle_and_palm;
pub mod office_and_markup;
pub mod pdf_tex_pptx;
pub mod web_and_archives;

pub use kindle_and_palm::{
    export_azw3_bytes, export_mobi_bytes, export_pdb_bytes, is_pdb_bytes, parse_pdb,
};
pub use office_and_markup::{
    export_adoc_bytes, export_docx_bytes, export_odt_bytes, export_org_bytes, export_pmlz_bytes,
    export_rb_bytes, export_rst_bytes, export_rtf_bytes, export_typst_bytes, is_adoc_input,
    is_org_input, is_rb_bytes, is_rst_input, is_typst_input, parse_adoc, parse_org,
    parse_pmlz_archive, parse_rb, parse_rst, parse_typst,
};
pub use pdf_tex_pptx::{
    export_pdf_bytes, export_pptx_bytes, export_tex_bytes, is_tex_input, parse_pptx, parse_tex,
};
pub use web_and_archives::{
    export_fb2_bytes, export_html_bytes, export_htmlz_bytes, export_kepub_bytes,
    export_markdown_bytes, export_oeb_bytes, export_oeb_dir, export_tcr_bytes, export_txt_bytes,
    export_txtz_bytes, export_zip_bytes, is_standalone_html, is_tcr_bytes, parse_html,
    parse_oeb_path, parse_tcr, parse_zip_ebook,
};
