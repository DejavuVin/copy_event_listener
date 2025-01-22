use std::fmt::Debug;
use unicode_segmentation::UnicodeSegmentation;

const MAX_DISPLAY_LENGTH: usize = 150;

#[derive(Debug)]
struct ClipboardTypedData {
    uti: String,
    data: Vec<u8>,
}

#[derive(Debug)]
pub struct ClipboardContent {
    item: Vec<ClipboardTypedData>,
}

impl ClipboardContent {
    pub fn new() -> Self {
        Self { item: Vec::new() }
    }

    pub fn on_data(&mut self, uti: String, data: Vec<u8>) {
        self.item.push(ClipboardTypedData { uti, data });
    }

    fn truncate_string(s: &str) -> String {
        let graphemes: Vec<&str> = s.graphemes(true).collect();
        if graphemes.len() > MAX_DISPLAY_LENGTH {
            format!("{}...", graphemes[..MAX_DISPLAY_LENGTH].join(""))
        } else {
            s.to_string()
        }
    }

    pub fn display_all(&self) {
        self.item
            .iter()
            .for_each(|x| println!("{}: {}", x.uti, Self::truncate_string(&String::from_utf8_lossy(&x.data))));
    }

    pub fn display(&self) -> String {
        // Check for image
        if let Some(img) = self.item.iter().find(|x| x.uti.contains("public.png")) {
            return format!("image, size: {} bytes", img.data.len());
        }

        // Check for HTML with plain text
        if let Some(_) = self.item.iter().find(|x| x.uti.contains("public.html")) {
            if let Some(text) = self.item.iter().find(|x| x.uti == "public.utf8-plain-text") {
                let content = String::from_utf8_lossy(&text.data);
                return format!("html, {}", Self::truncate_string(&content));
            }
            return "html, no plain text".to_string();
        }

        // Check for PDF
        if let Some(pdf) = self.item.iter().find(|x| x.uti.contains("com.adobe.pdf")) {
            return format!("pdf, size: {} bytes", pdf.data.len());
        }

        // Check for URL
        if let Some(url) = self.item.iter().find(|x| x.uti.contains("public.url")) {
            let content = String::from_utf8_lossy(&url.data);
            return format!("url: {}", Self::truncate_string(&content));
        }

        // Check for file URLs
        if let Some(file) = self.item.iter().find(|x| x.uti.contains("public.file-url")) {
            let content = String::from_utf8_lossy(&file.data);
            return format!("file: {}", Self::truncate_string(&content));
        }

        // Check for RTF
        if let Some(rtf) = self.item.iter().find(|x| x.uti.contains("public.rtf")) {
            if let Some(text) = self.item.iter().find(|x| x.uti == "public.utf8-plain-text") {
                let content = String::from_utf8_lossy(&text.data);
                return format!("rtf, {}", Self::truncate_string(&content));
            }
            return format!("rtf, size: {} bytes", rtf.data.len());
        }

        // Default case: plain text only
        if let Some(text) = self.item.iter().find(|x| x.uti == "public.utf8-plain-text") {
            let content = String::from_utf8_lossy(&text.data);
            return format!("text, {}", Self::truncate_string(&content));
        }

        // Fallback for unknown types
        if let Some(first) = self.item.first() {
            return format!("{}, size: {} bytes", first.uti, first.data.len());
        }

        "empty".to_string()
    }

    pub fn len(&self) -> usize {
        self.item.len()
    }
}
