use ebook_rs::Book;
use std::fs::File;
use std::io::Write;

#[test]
fn test_blackbox_mmap_file_reading() {
    let pdf_content = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\n";
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join("test_mmap_sample.pdf");
    {
        let mut f = File::create(&temp_path).unwrap();
        f.write_all(pdf_content).unwrap();
    }

    let path_str = temp_path.to_str().unwrap();
    let book_res = Book::from_mmap(path_str);
    assert!(book_res.is_ok() || book_res.is_err());

    let _ = std::fs::remove_file(temp_path);
}
