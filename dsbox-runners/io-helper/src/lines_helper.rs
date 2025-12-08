use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader, Lines, Result};

pub struct LinesHelper<R> {
    lines: Lines<BufReader<R>>,
    is_closed: bool,
}

impl<R> LinesHelper<R> {
    pub fn new(lines: R) -> LinesHelper<R>
    where
        R: AsyncRead,
    {
        LinesHelper {
            lines: BufReader::new(lines).lines(),
            is_closed: false,
        }
    }

    pub fn is_closed(&self) -> bool {
        self.is_closed
    }

    pub async fn line(&mut self) -> Result<Option<String>>
    where
        R: AsyncRead + Unpin,
    {
        let line = self.lines.next_line().await;
        self.is_closed = line.as_ref().map(Option::is_none).unwrap_or(true);
        line
    }
}
