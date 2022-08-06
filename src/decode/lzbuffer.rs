use crate::error;
use std::io;

pub trait LzBuffer<'a, W>
where
    W: io::Write,
{
    // Create a new buffer from the stream.
    fn from_stream(stream: W, dict_size: usize, memlimit: usize, buf: &'a mut Vec<u8>) -> Self;
    // Retrieve the length of the buffer
    fn len(&self) -> usize;
    // Retrieve the last byte or return a default
    fn last_or(&self, lit: u8) -> u8;
    // Retrieve the n-th last byte
    fn last_n(&self, dist: usize) -> error::Result<u8>;
    // Append a literal
    fn append_literal(&mut self, lit: u8) -> error::Result<()>;
    // Fetch an LZ sequence (length, distance) from inside the buffer
    fn append_lz(&mut self, len: usize, dist: usize) -> error::Result<()>;
    // Get a reference to the output sink
    fn get_output(&self) -> &W;
    // Get a mutable reference to the output sink
    fn get_output_mut(&mut self) -> &mut W;
    // Consumes this buffer and flushes any data, returning the output stream and the internal buffer.
    fn finish(self) -> io::Result<W>;
    // Consumes this buffer without flushing any data
    fn into_output(self) -> W;
}

/// An accumulating buffer for LZ sequences
#[derive(Debug)]
pub struct LzAccumBuffer<'a, W>
where
    W: io::Write,
{
    stream: W,            // Output sink
    buf: &'a mut Vec<u8>, // Buffer
    memlimit: usize,      // Buffer memory limit
    len: usize,           // Total number of bytes sent through the buffer
}

impl<'a, W> LzAccumBuffer<'a, W>
where
    W: io::Write,
{
    // Append bytes
    pub(crate) fn append_bytes(&mut self, buf: &[u8]) {
        self.buf.extend_from_slice(buf);
        self.len += buf.len();
    }

    // Reset the internal dictionary
    pub(crate) fn reset(&mut self) -> io::Result<()> {
        self.stream.write_all(self.buf.as_slice())?;
        self.buf.clear();
        self.len = 0;
        Ok(())
    }
}

impl<'a, W> LzBuffer<'a, W> for LzAccumBuffer<'a, W>
where
    W: io::Write,
{
    fn from_stream(stream: W, _dict_size: usize, memlimit: usize, buf: &'a mut Vec<u8>) -> Self {
        Self {
            stream,
            buf,
            memlimit,
            len: 0,
        }
    }

    fn len(&self) -> usize {
        self.len
    }

    // Retrieve the last byte or return a default
    fn last_or(&self, lit: u8) -> u8 {
        let buf_len = self.buf.len();
        if buf_len == 0 {
            lit
        } else {
            self.buf[buf_len - 1]
        }
    }

    // Retrieve the n-th last byte
    fn last_n(&self, dist: usize) -> error::Result<u8> {
        let buf_len = self.buf.len();
        if dist > buf_len {
            return Err(error::Error::LzmaError(format!(
                "Match distance {} is beyond output size {}",
                dist, buf_len
            )));
        }

        Ok(self.buf[buf_len - dist])
    }

    // Append a literal
    fn append_literal(&mut self, lit: u8) -> error::Result<()> {
        let new_len = self.len + 1;

        if new_len > self.memlimit {
            Err(error::Error::LzmaError(format!(
                "exceeded memory limit of {}",
                self.memlimit
            )))
        } else {
            self.buf.push(lit);
            self.len = new_len;
            Ok(())
        }
    }

    // Fetch an LZ sequence (length, distance) from inside the buffer
    fn append_lz(&mut self, len: usize, dist: usize) -> error::Result<()> {
        lzma_debug!("LZ {{ len: {}, dist: {} }}", len, dist);
        let buf_len = self.buf.len();
        if dist > buf_len {
            return Err(error::Error::LzmaError(format!(
                "LZ distance {} is beyond output size {}",
                dist, buf_len
            )));
        }

        let mut offset = buf_len - dist;
        for _ in 0..len {
            let x = self.buf[offset];
            self.buf.push(x);
            offset += 1;
        }
        self.len += len;
        Ok(())
    }

    // Get a reference to the output sink
    fn get_output(&self) -> &W {
        &self.stream
    }

    // Get a mutable reference to the output sink
    fn get_output_mut(&mut self) -> &mut W {
        &mut self.stream
    }

    // Consumes this buffer and flushes any data
    fn finish(mut self) -> io::Result<W> {
        self.stream.write_all(self.buf.as_slice())?;
        self.stream.flush()?;
        Ok(self.stream)
    }

    // Consumes this buffer without flushing any data
    fn into_output(self) -> W {
        self.stream
    }
}

/// A circular buffer for LZ sequences
#[derive(Debug)]
pub struct LzCircularBuffer<'a, W>
where
    W: io::Write,
{
    stream: W,            // Output sink
    buf: &'a mut Vec<u8>, // Circular buffer
    dict_size: usize,     // Length of the buffer
    memlimit: usize,      // Buffer memory limit
    cursor: usize,        // Current position
    len: usize,           // Total number of bytes sent through the buffer
}

impl<'a, W> LzCircularBuffer<'a, W>
where
    W: io::Write,
{
    fn get(&self, index: usize) -> u8 {
        *self.buf.get(index).unwrap_or(&0)
    }

    fn set(&mut self, index: usize, value: u8) -> error::Result<()> {
        let new_len = index + 1;

        if self.buf.len() < new_len {
            if new_len <= self.memlimit {
                self.buf.resize(new_len, 0);
            } else {
                return Err(error::Error::LzmaError(format!(
                    "exceeded memory limit of {}",
                    self.memlimit
                )));
            }
        }
        self.buf[index] = value;
        Ok(())
    }
}

impl<'a, W> LzBuffer<'a, W> for LzCircularBuffer<'a, W>
where
    W: io::Write,
{
    fn from_stream(stream: W, dict_size: usize, memlimit: usize, buf: &'a mut Vec<u8>) -> Self {
        lzma_info!("Dict size in LZ buffer: {}", dict_size);
        Self {
            stream,
            buf,
            dict_size,
            memlimit,
            cursor: 0,
            len: 0,
        }
    }

    fn len(&self) -> usize {
        self.len
    }

    // Retrieve the last byte or return a default
    fn last_or(&self, lit: u8) -> u8 {
        if self.len == 0 {
            lit
        } else {
            self.get((self.dict_size + self.cursor - 1) % self.dict_size)
        }
    }

    // Retrieve the n-th last byte
    fn last_n(&self, dist: usize) -> error::Result<u8> {
        if dist > self.dict_size {
            return Err(error::Error::LzmaError(format!(
                "Match distance {} is beyond dictionary size {}",
                dist, self.dict_size
            )));
        }
        if dist > self.len {
            return Err(error::Error::LzmaError(format!(
                "Match distance {} is beyond output size {}",
                dist, self.len
            )));
        }

        let offset = (self.dict_size + self.cursor - dist) % self.dict_size;
        Ok(self.get(offset))
    }

    // Append a literal
    fn append_literal(&mut self, lit: u8) -> error::Result<()> {
        self.set(self.cursor, lit)?;
        self.cursor += 1;
        self.len += 1;

        // Flush the circular buffer to the output
        if self.cursor == self.dict_size {
            self.stream.write_all(self.buf.as_slice())?;
            self.cursor = 0;
        }

        Ok(())
    }

    // Fetch an LZ sequence (length, distance) from inside the buffer
    fn append_lz(&mut self, len: usize, dist: usize) -> error::Result<()> {
        lzma_debug!("LZ {{ len: {}, dist: {} }}", len, dist);
        if dist > self.dict_size {
            return Err(error::Error::LzmaError(format!(
                "LZ distance {} is beyond dictionary size {}",
                dist, self.dict_size
            )));
        }
        if dist > self.len {
            return Err(error::Error::LzmaError(format!(
                "LZ distance {} is beyond output size {}",
                dist, self.len
            )));
        }

        let mut offset = (self.dict_size + self.cursor - dist) % self.dict_size;
        for _ in 0..len {
            let x = self.get(offset);
            self.append_literal(x)?;
            offset += 1;
            if offset == self.dict_size {
                offset = 0
            }
        }
        Ok(())
    }

    // Get a reference to the output sink
    fn get_output(&self) -> &W {
        &self.stream
    }

    // Get a mutable reference to the output sink
    fn get_output_mut(&mut self) -> &mut W {
        &mut self.stream
    }

    // Consumes this buffer and flushes any data
    fn finish(mut self) -> io::Result<W> {
        if self.cursor > 0 {
            self.stream.write_all(&self.buf[0..self.cursor])?;
            self.stream.flush()?;
        }
        Ok(self.stream)
    }

    // Consumes this buffer without flushing any data
    fn into_output(self) -> W {
        self.stream
    }
}
