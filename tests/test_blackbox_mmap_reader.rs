use ebook_rs::Book;
use std::fs::File;
use std::io::Write;

#[test]
fn test_blackbox_mmap_file_reading() {
    let md_content = b"# Mmap Test\n\nSample content for memory-mapped book parsing.";
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join("test_mmap_sample.md");
    {
        let mut f = File::create(&temp_path).unwrap();
        f.write_all(md_content).unwrap();
    }

    let path_str = temp_path.to_str().unwrap();
    let book_res = Book::from_mmap(path_str);
    assert!(book_res.is_ok());
    let book = book_res.unwrap();
    assert_eq!(book.metadata().title, "Mmap Test");

    let _ = std::fs::remove_file(temp_path);
}
