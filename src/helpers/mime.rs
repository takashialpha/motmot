pub fn from_bytes(data: &[u8]) -> &'static str {
    mimetype_detector::detect(data).mime()
}
