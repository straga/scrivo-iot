//! The exchange over a socket that is only a Read and a Write: what the board sends, and how an answer that comes in pieces, too long, or not at all is taken.
//! The socket here is a stand-in with a script; tests/over_tcp.rs does the same over real sockets.

use embassy_futures::block_on;
use embedded_io_async::{ErrorKind, ErrorType, Read, Write};
use tasmota::{exchange, Fault, REQUEST};

struct Scripted {
    answer: Vec<u8>,
    at: usize,
    piece: usize,
    written: Vec<u8>,
    reset_after: Option<usize>,
}

impl Scripted {
    fn saying(answer: &[u8], piece: usize) -> Self {
        Scripted { answer: answer.to_vec(), at: 0, piece, written: Vec::new(), reset_after: None }
    }
}

impl ErrorType for Scripted {
    type Error = ErrorKind;
}

impl Read for Scripted {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, ErrorKind> {
        if self.reset_after.is_some_and(|n| self.at >= n) {
            return Err(ErrorKind::ConnectionReset);
        }
        let n = self.piece.min(buf.len()).min(self.answer.len() - self.at);
        buf[..n].copy_from_slice(&self.answer[self.at..self.at + n]);
        self.at += n;
        Ok(n)
    }
}

impl Write for Scripted {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, ErrorKind> {
        // a socket that takes a few bytes at a time: the request must still arrive whole
        let n = buf.len().min(9);
        self.written.extend_from_slice(&buf[..n]);
        Ok(n)
    }
    async fn flush(&mut self) -> Result<(), ErrorKind> {
        Ok(())
    }
}

const ANSWER: &[u8] = b"HTTP/1.0 200 OK\r\nConnection: close\r\n\r\n{\"StatusSNS\":{\"ENERGY\":{\"Power\":3,\"Voltage\":227,\"Current\":0.060}}}";

#[test]
fn the_request_that_goes_out_is_the_whole_of_one_read_of_the_status_and_nothing_else() {
    let mut socket = Scripted::saying(ANSWER, 1000);
    let mut buf = [0u8; 512];
    block_on(exchange(&mut socket, &mut buf)).unwrap();
    assert_eq!(socket.written, REQUEST);
    let text = std::str::from_utf8(REQUEST).unwrap();
    assert!(text.starts_with("GET /cm?cmnd=Status%2010 HTTP/1.0\r\n"), "{text}");
    assert!(text.ends_with("\r\n\r\n"), "{text}");
    // the socket powers the whole burner: a command to it is never among the words of this request
    assert!(!text.to_ascii_lowercase().contains("cmnd=power"), "{text}");
    assert_eq!(text.matches("GET ").count(), 1);
}

#[test]
fn an_answer_that_comes_in_small_pieces_is_taken_whole_up_to_the_close() {
    let mut socket = Scripted::saying(ANSWER, 7);
    let mut buf = [0u8; 512];
    let n = block_on(exchange(&mut socket, &mut buf)).unwrap();
    assert_eq!(&buf[..n], ANSWER);
}

#[test]
fn an_answer_that_fills_the_buffer_exactly_is_still_whole() {
    let mut socket = Scripted::saying(ANSWER, 1000);
    let mut buf = vec![0u8; ANSWER.len()];
    let n = block_on(exchange(&mut socket, &mut buf)).unwrap();
    assert_eq!(n, ANSWER.len());
}

#[test]
fn an_answer_longer_than_the_buffer_is_too_long_and_not_cut_and_read() {
    let mut socket = Scripted::saying(ANSWER, 1000);
    let mut buf = vec![0u8; ANSWER.len() - 1];
    assert_eq!(block_on(exchange(&mut socket, &mut buf)), Err(Fault::TooLong));
}

#[test]
fn a_socket_that_closes_without_a_word_is_a_closed_one() {
    let mut socket = Scripted::saying(b"", 1000);
    let mut buf = [0u8; 512];
    assert_eq!(block_on(exchange(&mut socket, &mut buf)), Err(Fault::Closed));
}

#[test]
fn a_connection_reset_in_the_middle_of_the_answer_is_silence_and_not_a_half_reading() {
    let mut socket = Scripted::saying(ANSWER, 10);
    socket.reset_after = Some(30);
    let mut buf = [0u8; 512];
    assert_eq!(block_on(exchange(&mut socket, &mut buf)), Err(Fault::Silent));
}
