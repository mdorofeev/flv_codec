flv_codec
=========

Decoders and encoders for [FLV] file format.


Examples
--------

```rust
# extern crate bytecodec;
# extern crate flv_codec;
use bytecodec::io::IoDecodeExt;
use flv_codec::{FileDecoder, Header, Tag};

// Reads FLV file content
let mut flv = &include_bytes!("../black_silent.flv")[..];
let mut decoder = FileDecoder::new();

// Decodes the first FLV tag
let tag = decoder.decode_exact(&mut flv).unwrap();
let header = decoder.header().cloned().unwrap();
assert_eq!(header, Header { has_audio: true, has_video: true });
assert_eq!(tag.timestamp().value(), 0);
assert_eq!(tag.stream_id().value(), 0);
match tag {
    Tag::Audio(_) => println!("audio tag"),
    Tag::Video(_) => println!("video tag"),
    Tag::ScriptData(_) => println!("script data tag"),
}

// Decodes the second FLV tag
let tag = decoder.decode_exact(&mut flv).unwrap();
```

See [examples/] directory for more examples.


References
-----------

- [Video File Format Specification][FLV]

[FLV]: https://wwwimages2.adobe.com/content/dam/acom/en/devnet/flv/video_file_format_spec_v10.pdf
[examples/]: https://github.com/sile/flv_codec/tree/master/examples
