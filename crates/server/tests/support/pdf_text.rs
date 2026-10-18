//! Poppler interprets embedded Unicode font maps that lopdf 0.26 cannot decode.
pub fn text(bytes: &[u8]) -> String {
    use std::io::Write;
    let mut child = std::process::Command::new("pdftotext")
        .args(["-enc", "UTF-8", "-", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Install Poppler (pdftotext) for PDF output integration tests");
    child.stdin.take().unwrap().write_all(bytes).unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "pdftotext failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout).unwrap()
}
