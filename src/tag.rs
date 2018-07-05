use bytecodec::bytes::RemainingBytesDecoder;
use bytecodec::combinator::{Length, Peekable};
use bytecodec::fixnum::{U24beDecoder, U8Decoder};
use bytecodec::{ByteCount, Decode, DecodeExt, Eos, ErrorKind, Result};

const TAG_TYPE_AUDIO: u8 = 8;
const TAG_TYPE_VIDEO: u8 = 9;
const TAG_TYPE_SCRIPT_DATA: u8 = 18;

#[derive(Debug)]
pub enum FlvTag {
    Audio(AudioTag),
    Video(VideoTag),
    ScriptData(ScriptDataTag),
}
impl FlvTag {
    pub fn tag_type(&self) -> TagType {
        match self {
            FlvTag::Audio(_) => TagType::Audio,
            FlvTag::Video(_) => TagType::Video,
            FlvTag::ScriptData(_) => TagType::ScriptData,
        }
    }

    pub fn timestamp(&self) -> Timestamp {
        match self {
            FlvTag::Audio(t) => t.timestamp,
            FlvTag::Video(t) => t.timestamp,
            FlvTag::ScriptData(t) => t.timestamp,
        }
    }

    pub fn stream_id(&self) -> StreamId {
        match self {
            FlvTag::Audio(t) => t.stream_id,
            FlvTag::Video(t) => t.stream_id,
            FlvTag::ScriptData(t) => t.stream_id,
        }
    }
}
impl From<AudioTag> for FlvTag {
    fn from(f: AudioTag) -> Self {
        FlvTag::Audio(f)
    }
}
impl From<VideoTag> for FlvTag {
    fn from(f: VideoTag) -> Self {
        FlvTag::Video(f)
    }
}
impl From<ScriptDataTag> for FlvTag {
    fn from(f: ScriptDataTag) -> Self {
        FlvTag::ScriptData(f)
    }
}

#[derive(Debug)]
pub struct AudioTag {
    pub timestamp: Timestamp,
    pub stream_id: StreamId,
}

#[derive(Debug)]
pub struct VideoTag {
    pub timestamp: Timestamp,
    pub stream_id: StreamId,
    pub frame_type: FrameType,
    pub codec_id: CodecId,
    pub avc_packet_type: Option<AvcPacketType>,
    pub composition_time: Option<CompositionTimeOffset>,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct ScriptDataTag {
    pub timestamp: Timestamp,
    pub stream_id: StreamId,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TagType {
    Audio = TAG_TYPE_AUDIO as isize,
    Video = TAG_TYPE_VIDEO as isize,
    ScriptData = TAG_TYPE_SCRIPT_DATA as isize,
}

// TODO: move
// TODO: to/from Duration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Timestamp(i32);

// TODO: move
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CompositionTimeOffset(i32); // i24
impl CompositionTimeOffset {
    fn from_u24(n: u32) -> Self {
        // TODO: test
        CompositionTimeOffset(((n << 8) as i32) >> 8)
    }
}

// TODO: move
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StreamId(u32); // u24

#[derive(Debug, Default)]
pub struct FlvTagDecoder {
    header: Peekable<TagHeaderDecoder>,
    data: Length<TagDataDecoder>,
}
impl Decode for FlvTagDecoder {
    type Item = FlvTag;

    fn decode(&mut self, buf: &[u8], eos: Eos) -> Result<usize> {
        let mut offset = 0;
        if !self.header.is_idle() {
            bytecodec_try_decode!(self.header, offset, buf, eos);
            let header = self.header.peek().expect("Never fails");
            let data = match header.tag_type {
                TagType::Audio => TagDataDecoder::Audio(Default::default()),
                TagType::Video => TagDataDecoder::Video(Default::default()),
                TagType::ScriptData => TagDataDecoder::ScriptData(Default::default()),
            };
            self.data = data.length(u64::from(header.data_size));
        }
        bytecodec_try_decode!(self.data, offset, buf, eos);
        Ok(offset)
    }

    fn finish_decoding(&mut self) -> Result<Self::Item> {
        let header = track!(self.header.finish_decoding())?;
        let data = track!(self.data.finish_decoding())?;
        let tag = match data {
            TagData::Audio(_) => panic!(),
            TagData::Video(d) => FlvTag::from(VideoTag {
                timestamp: header.timestamp,
                stream_id: header.stream_id,
                frame_type: d.frame_type,
                codec_id: d.codec_id,
                avc_packet_type: d.avc_packet_type,
                composition_time: d.composition_time,
                data: d.data,
            }),
            TagData::ScriptData(d) => FlvTag::from(ScriptDataTag {
                timestamp: header.timestamp,
                stream_id: header.stream_id,
                data: d.data,
            }),
        };
        Ok(tag)
    }

    fn is_idle(&self) -> bool {
        self.header.is_idle() && self.data.is_idle()
    }

    fn requiring_bytes(&self) -> ByteCount {
        if self.header.is_idle() {
            self.data.requiring_bytes()
        } else {
            self.header.requiring_bytes()
        }
    }
}

#[derive(Debug)]
struct TagHeader {
    tag_type: TagType,
    data_size: u32, // u24
    timestamp: Timestamp,
    stream_id: StreamId,
}

#[derive(Debug, Default)]
struct TagHeaderDecoder {
    tag_type: U8Decoder,
    data_size: U24beDecoder,
    timestamp: U24beDecoder,
    timestamp_extended: U8Decoder,
    stream_id: U24beDecoder,
}
impl Decode for TagHeaderDecoder {
    type Item = TagHeader;

    fn decode(&mut self, buf: &[u8], eos: Eos) -> Result<usize> {
        let mut offset = 0;
        bytecodec_try_decode!(self.tag_type, offset, buf, eos);
        bytecodec_try_decode!(self.data_size, offset, buf, eos);
        bytecodec_try_decode!(self.timestamp, offset, buf, eos);
        bytecodec_try_decode!(self.timestamp_extended, offset, buf, eos);
        bytecodec_try_decode!(self.stream_id, offset, buf, eos);
        Ok(offset)
    }

    fn finish_decoding(&mut self) -> Result<Self::Item> {
        let tag_type = track!(self.tag_type.finish_decoding())?;
        let data_size = track!(self.data_size.finish_decoding())?;
        let timestamp = track!(self.timestamp.finish_decoding())?;
        let timestamp_extended = track!(self.timestamp_extended.finish_decoding())?;
        let stream_id = track!(self.stream_id.finish_decoding())?;

        let tag_type = match tag_type {
            TAG_TYPE_AUDIO => TagType::Audio,
            TAG_TYPE_VIDEO => TagType::Video,
            TAG_TYPE_SCRIPT_DATA => TagType::ScriptData,
            _ => track_panic!(
                ErrorKind::InvalidInput,
                "Unknown FLV tag type: {}",
                tag_type
            ),
        };
        track_assert!(
            data_size <= 0x00FF_FFFF,
            ErrorKind::InvalidInput,
            "Too large FLV tag data size: {}",
            data_size
        );
        let timestamp = Timestamp((timestamp as i32) | i32::from(timestamp_extended) << 24);
        Ok(TagHeader {
            tag_type,
            data_size,
            timestamp,
            stream_id: StreamId(stream_id),
        })
    }

    fn is_idle(&self) -> bool {
        self.stream_id.is_idle()
    }

    fn requiring_bytes(&self) -> ByteCount {
        self.tag_type
            .requiring_bytes()
            .add_for_decoding(self.data_size.requiring_bytes())
            .add_for_decoding(self.timestamp.requiring_bytes())
            .add_for_decoding(self.timestamp_extended.requiring_bytes())
            .add_for_decoding(self.stream_id.requiring_bytes())
    }
}

#[derive(Debug)]
enum TagData {
    Audio(AudioTagData),
    Video(VideoTagData),
    ScriptData(ScriptDataTagData),
}

#[derive(Debug)]
struct AudioTagData;

#[derive(Debug)]
pub enum SoundFormat {
    LinearPcmPlatformEndian = 0,
    Adpcm = 1,
    Mp3 = 2,
    LinearPcmLittleEndian = 3,
    Nellymoser16khzMono = 4,
    Nellymoser8KhzMono = 5,
    Nellymoser = 6,
    G711AlawLogarithmicPcm = 7,
    G711MuLawLogarithmicPcm = 8,
    Aac = 10,
    Speex = 11,
    Mp3_8khz = 14,
    DeviceSpecificSound = 15,
}

#[derive(Debug)]
struct VideoTagData {
    frame_type: FrameType,
    codec_id: CodecId,
    avc_packet_type: Option<AvcPacketType>,
    composition_time: Option<CompositionTimeOffset>,
    data: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameType {
    KeyFrame = 1,
    InterFrame = 2,
    DisposableInterFrame = 3,
    GeneratedKeyFrame = 4,
    VideoInfoOrCommandFrame = 5,
}
impl FrameType {
    fn from_u8(b: u8) -> Result<Self> {
        Ok(match b {
            1 => FrameType::KeyFrame,
            2 => FrameType::InterFrame,
            3 => FrameType::DisposableInterFrame,
            4 => FrameType::GeneratedKeyFrame,
            5 => FrameType::VideoInfoOrCommandFrame,
            _ => track_panic!(ErrorKind::InvalidInput, "Unknown video frame type: {}", b),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CodecId {
    Jpeg = 1,
    H263 = 2,
    ScreenVideo = 3,
    Vp6 = 4,
    Vp6WithAlpha = 5,
    ScreenVideoV2 = 6,
    Avc = 7,
}
impl CodecId {
    fn from_u8(b: u8) -> Result<Self> {
        Ok(match b {
            1 => CodecId::Jpeg,
            2 => CodecId::H263,
            3 => CodecId::ScreenVideo,
            4 => CodecId::Vp6,
            5 => CodecId::Vp6WithAlpha,
            6 => CodecId::ScreenVideoV2,
            7 => CodecId::Avc,
            _ => track_panic!(ErrorKind::InvalidInput, "Unknown video codec ID: {}", b),
        })
    }
}

#[derive(Debug)]
pub enum AvcPacketType {
    SequenceHeader = 0,
    NalUnit = 1,
    EndOfSequence = 2,
}
impl AvcPacketType {
    fn from_u8(b: u8) -> Result<Self> {
        Ok(match b {
            0 => AvcPacketType::SequenceHeader,
            1 => AvcPacketType::NalUnit,
            2 => AvcPacketType::EndOfSequence,
            _ => track_panic!(ErrorKind::InvalidInput, "Unknown AVC packet type: {}", b),
        })
    }
}

#[derive(Debug)]
struct ScriptDataTagData {
    data: Vec<u8>,
}

#[derive(Debug)]
enum TagDataDecoder {
    Audio(AudioTagDataDecoder),
    Video(VideoTagDataDecoder),
    ScriptData(ScriptDataTagDataDecoder),
    None,
}
impl Decode for TagDataDecoder {
    type Item = TagData;

    fn decode(&mut self, buf: &[u8], eos: Eos) -> Result<usize> {
        match self {
            TagDataDecoder::Audio(d) => track!(d.decode(buf, eos)),
            TagDataDecoder::Video(d) => track!(d.decode(buf, eos)),
            TagDataDecoder::ScriptData(d) => track!(d.decode(buf, eos)),
            TagDataDecoder::None => track_panic!(ErrorKind::InconsistentState),
        }
    }

    fn finish_decoding(&mut self) -> Result<Self::Item> {
        let data = match self {
            TagDataDecoder::Audio(d) => TagData::Audio(track!(d.finish_decoding())?),
            TagDataDecoder::Video(d) => TagData::Video(track!(d.finish_decoding())?),
            TagDataDecoder::ScriptData(d) => TagData::ScriptData(track!(d.finish_decoding())?),
            TagDataDecoder::None => track_panic!(ErrorKind::InconsistentState),
        };
        *self = TagDataDecoder::None;
        Ok(data)
    }

    fn is_idle(&self) -> bool {
        match self {
            TagDataDecoder::Audio(d) => d.is_idle(),
            TagDataDecoder::Video(d) => d.is_idle(),
            TagDataDecoder::ScriptData(d) => d.is_idle(),
            TagDataDecoder::None => true,
        }
    }

    fn requiring_bytes(&self) -> ByteCount {
        match self {
            TagDataDecoder::Audio(d) => d.requiring_bytes(),
            TagDataDecoder::Video(d) => d.requiring_bytes(),
            TagDataDecoder::ScriptData(d) => d.requiring_bytes(),
            TagDataDecoder::None => ByteCount::Finite(0),
        }
    }
}
impl Default for TagDataDecoder {
    fn default() -> Self {
        TagDataDecoder::None
    }
}

#[derive(Debug, Default)]
struct AudioTagDataDecoder {}
impl Decode for AudioTagDataDecoder {
    type Item = AudioTagData;

    fn decode(&mut self, buf: &[u8], eos: Eos) -> Result<usize> {
        panic!()
    }

    fn finish_decoding(&mut self) -> Result<Self::Item> {
        panic!()
    }

    fn is_idle(&self) -> bool {
        panic!()
    }

    fn requiring_bytes(&self) -> ByteCount {
        panic!()
    }
}

#[derive(Debug, Default)]
struct FrameTypeAndCodecDecoder(U8Decoder);
impl Decode for FrameTypeAndCodecDecoder {
    type Item = (FrameType, CodecId);

    fn decode(&mut self, buf: &[u8], eos: Eos) -> Result<usize> {
        track!(self.0.decode(buf, eos))
    }

    fn finish_decoding(&mut self) -> Result<Self::Item> {
        let b = track!(self.0.finish_decoding())?;
        let frame_type = track!(FrameType::from_u8(b >> 4))?;
        let codec_id = track!(CodecId::from_u8(b & 0b1111))?;
        Ok((frame_type, codec_id))
    }

    fn is_idle(&self) -> bool {
        self.0.is_idle()
    }

    fn requiring_bytes(&self) -> ByteCount {
        self.0.requiring_bytes()
    }
}

#[derive(Debug, Default)]
struct VideoTagDataDecoder {
    frame_type_and_codec: Peekable<FrameTypeAndCodecDecoder>,
    avc_packet_type: U8Decoder,
    composition_time: U24beDecoder,
    data: RemainingBytesDecoder,
}
impl VideoTagDataDecoder {
    fn is_avc_packet(&self) -> bool {
        self.frame_type_and_codec.peek().map_or(false, |t| {
            t.0 != FrameType::VideoInfoOrCommandFrame && t.1 == CodecId::Avc
        })
    }
}
impl Decode for VideoTagDataDecoder {
    type Item = VideoTagData;

    fn decode(&mut self, buf: &[u8], eos: Eos) -> Result<usize> {
        let mut offset = 0;
        bytecodec_try_decode!(self.frame_type_and_codec, offset, buf, eos);
        if self.is_avc_packet() {
            bytecodec_try_decode!(self.avc_packet_type, offset, buf, eos);
            bytecodec_try_decode!(self.composition_time, offset, buf, eos);
        }
        bytecodec_try_decode!(self.data, offset, buf, eos);
        Ok(offset)
    }

    fn finish_decoding(&mut self) -> Result<Self::Item> {
        let is_avc_packet = self.is_avc_packet();

        let (frame_type, codec_id) = track!(self.frame_type_and_codec.finish_decoding())?;
        let avc_packet_type = if is_avc_packet {
            let avc_packet_type = track!(self.avc_packet_type.finish_decoding())?;
            let avc_packet_type = track!(AvcPacketType::from_u8(avc_packet_type))?;
            Some(avc_packet_type)
        } else {
            None
        };
        let composition_time = if is_avc_packet {
            let composition_time = track!(self.composition_time.finish_decoding())?;
            let composition_time = CompositionTimeOffset::from_u24(composition_time);
            Some(composition_time)
        } else {
            None
        };
        let data = track!(self.data.finish_decoding())?;
        Ok(VideoTagData {
            frame_type,
            codec_id,
            avc_packet_type,
            composition_time,
            data,
        })
    }

    fn is_idle(&self) -> bool {
        self.data.is_idle()
    }

    fn requiring_bytes(&self) -> ByteCount {
        ByteCount::Unknown
    }
}

#[derive(Debug, Default)]
struct ScriptDataTagDataDecoder(RemainingBytesDecoder);
impl Decode for ScriptDataTagDataDecoder {
    type Item = ScriptDataTagData;

    fn decode(&mut self, buf: &[u8], eos: Eos) -> Result<usize> {
        track!(self.0.decode(buf, eos))
    }

    fn finish_decoding(&mut self) -> Result<Self::Item> {
        let data = track!(self.0.finish_decoding())?;
        Ok(ScriptDataTagData { data })
    }

    fn is_idle(&self) -> bool {
        self.0.is_idle()
    }

    fn requiring_bytes(&self) -> ByteCount {
        self.0.requiring_bytes()
    }
}