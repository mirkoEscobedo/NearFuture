use crate::IpcError;
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedFrame {
    pub consumed: usize,
    pub frame: Option<Vec<u8>>,
}
/// Incremental foreground-independent decoder. Length is checked before body allocation.
pub struct FrameDecoder {
    maximum: usize,
    header: [u8; 4],
    header_used: usize,
    length: Option<usize>,
    body: zeroize::Zeroizing<Vec<u8>>,
}
impl FrameDecoder {
    pub fn new(maximum: usize) -> Result<Self, IpcError> {
        if maximum == 0 || maximum > 1_048_576 {
            return Err(IpcError::Limit);
        }
        Ok(Self {
            maximum,
            header: [0; 4],
            header_used: 0,
            length: None,
            body: zeroize::Zeroizing::new(Vec::new()),
        })
    }
    pub fn push(&mut self, input: &[u8]) -> Result<DecodedFrame, IpcError> {
        let mut consumed = 0;
        if self.length.is_none() {
            let take = (4 - self.header_used).min(input.len());
            self.header[self.header_used..self.header_used + take].copy_from_slice(&input[..take]);
            self.header_used += take;
            consumed += take;
            if self.header_used < 4 {
                return Ok(DecodedFrame {
                    consumed,
                    frame: None,
                });
            }
            let length = u32::from_be_bytes(self.header) as usize;
            if length == 0 {
                self.reset();
                return Err(IpcError::Malformed);
            }
            if length > self.maximum {
                self.reset();
                return Err(IpcError::Limit);
            }
            self.length = Some(length);
            self.body = zeroize::Zeroizing::new(Vec::with_capacity(length));
        }
        let length = self.length.ok_or(IpcError::Malformed)?;
        let take = (length - self.body.len()).min(input.len() - consumed);
        self.body
            .extend_from_slice(&input[consumed..consumed + take]);
        consumed += take;
        let frame = if self.body.len() == length {
            let frame = std::mem::take(&mut *self.body);
            self.header_used = 0;
            self.length = None;
            Some(frame)
        } else {
            None
        };
        Ok(DecodedFrame { consumed, frame })
    }
    pub fn finish(&self) -> Result<(), IpcError> {
        if self.header_used != 0 || self.length.is_some() {
            Err(IpcError::Incomplete)
        } else {
            Ok(())
        }
    }
    pub fn reset(&mut self) {
        self.header = [0; 4];
        self.header_used = 0;
        self.length = None;
        self.body.clear();
    }
    pub fn needed_bytes(&self) -> usize {
        self.length
            .map_or(4 - self.header_used, |length| length - self.body.len())
    }
    pub fn buffered_bytes(&self) -> usize {
        self.header_used + self.body.len()
    }
}
pub fn encode_frame(body: &[u8], maximum: usize) -> Result<Vec<u8>, IpcError> {
    if maximum == 0 || maximum > 1_048_576 || body.len() > maximum {
        return Err(IpcError::Limit);
    }
    if body.is_empty() {
        return Err(IpcError::Malformed);
    }
    let mut frame = Vec::with_capacity(body.len() + 4);
    frame.extend_from_slice(&(body.len() as u32).to_be_bytes());
    frame.extend_from_slice(body);
    Ok(frame)
}
