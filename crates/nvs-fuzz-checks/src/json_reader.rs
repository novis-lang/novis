use nvs_stdlib::fuzz::{json_read, json_reader_disagreement};

/// `Core\Json`'s reader must not panic on any bytes, must read them the same
/// from one text and from chunks of any size, and must agree with
/// `serde_json`: the same tokens and the same error text, with the exceptions
/// `json_reader_disagreement` names.
pub fn json_reader(data: &[u8]) {
    let whole = json_read(data, None);
    for size in [1, 2, 3, 7, 64] {
        assert_eq!(
            json_read(data, Some(size)),
            whole,
            "a read in chunks of {size} bytes gives what one text gives"
        );
    }
    if let Some(why) = json_reader_disagreement(data) {
        panic!("the reader and serde_json disagree: {why}");
    }
}
