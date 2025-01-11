use super::tools::{ArxivError, Paper};
use core::str;
use quick_xml::{
    Reader,
    events::{BytesEnd, BytesStart, BytesText, Event},
};

#[derive(Debug, Default)]
pub struct ArxivParser<'a> {
    pub papers: Vec<Paper>,
    pub current_paper: Option<Paper>,
    pub current_authors: Vec<String>,
    pub current_categories: Vec<String>,
    pub in_entry: bool,
    pub current_field: Option<&'a str>,
}

impl<'a> ArxivParser<'a> {
    fn parse_start_event(&mut self, event: &BytesStart) {
        match event.name().as_ref() {
            b"entry" => {
                self.in_entry = true;
                self.current_paper = Some(Paper::default());
                self.current_authors.clear();
                self.current_categories.clear();
            }
            b"title" if self.in_entry => self.current_field = Some("title"),
            b"author" if self.in_entry => self.current_field = Some("author"),
            b"summary" if self.in_entry => self.current_field = Some("abstract"),
            b"link" if self.in_entry => self.current_field = Some("link"),
            b"category" if self.in_entry => self.current_field = Some("category"),
            _ => (),
        };
    }

    fn parse_text_event(&mut self, event: &BytesText) -> Result<(), ArxivError> {
        let Some(paper) = self.current_paper.as_mut() else {
            return Ok(());
        };
        let text = str::from_utf8(event.as_ref())?.to_owned();
        match self.current_field {
            Some("title") => paper.title = text,
            Some("author") => self.current_authors.push(text),
            Some("abstract") => paper.abstract_text = text,
            _ => (),
        }
        Ok(())
    }

    fn parse_empty_event(&mut self, event: &BytesStart) -> Result<(), ArxivError> {
        if !self.in_entry {
            return Ok(());
        }

        if event.name().as_ref() == b"link" {
            if let Some(paper) = self.current_paper.as_mut() {
                for attr in event.attributes().flatten() {
                    if attr.key.as_ref() == b"href" {
                        let url = str::from_utf8(&attr.value)?;
                        // Convert to HTTPS and ensure PDF URL
                        let secure_url = convert_pdf_url(url);
                        secure_url.clone_into(&mut paper.url);
                    }
                }
            }
        }

        if event.name().as_ref() == b"category" {
            for attr in event.attributes().flatten() {
                if attr.key.as_ref() == b"term" {
                    self.current_categories
                        .push(str::from_utf8(&attr.value)?.to_owned());
                }
            }
        }

        Ok(())
    }

    fn parse_end_event(&mut self, event: &BytesEnd) -> Result<(), ArxivError> {
        match event.name().as_ref() {
            b"entry" => {
                if let Some(mut paper) = self.current_paper.take() {
                    paper.authors.clone_from(&self.current_authors);
                    paper.categories.clone_from(&self.current_categories);
                    self.papers.push(paper);
                }
                self.in_entry = false;
            }
            b"title" | b"author" | b"summary" | b"link" | b"category" => {
                self.current_field = None;
            }
            _ => (),
        }
        Ok(())
    }

    pub fn parse_response(&mut self, input: &str) -> Result<Vec<Paper>, ArxivError> {
        let mut reader = Reader::from_str(input);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => self.parse_start_event(e),
                Ok(Event::Text(ref e)) => self.parse_text_event(e)?,
                Ok(Event::Empty(ref e)) => self.parse_empty_event(e)?,
                Ok(Event::End(ref e)) => self.parse_end_event(e)?,
                Ok(Event::Eof) => break,
                Err(e) => return Err(ArxivError::XmlParsing(e)),
                _ => (),
            }
        }

        if self.papers.is_empty() {
            return Err(ArxivError::NoResults);
        }

        Ok(self.papers.clone())
    }
}

fn convert_pdf_url(url: &str) -> String {
    if url.contains("arxiv.org/abs/") {
        // Convert abstract URL to PDF URL
        url.replace("arxiv.org/abs/", "arxiv.org/pdf/")
            .replace("http://", "https://")
            + ".pdf"
    } else if url.contains("arxiv.org/pdf/") {
        // Ensure PDF URL uses HTTPS
        url.replace("http://", "https://")
    } else {
        // Fallback for other URLs
        url.replace("http://", "https://")
    }
}
