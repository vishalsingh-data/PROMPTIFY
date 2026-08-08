//! Encoded-payload detection and decoding engine for `promptify-core`.
//!
//! **Owns**: detecting and decoding encoded attack payloads embedded in prompts
//!           (Base64, URL-encoding, Unicode homoglyphs, ROT-13, hex sequences)
//!           and producing plaintext `DecodedPayload`s for re-evaluation by
//!           `RuleEngine`.
//! **Does not own**: rule evaluation on decoded text (→ `rules`), risk scoring
//!                   (→ `scoring`), or persistence (→ `logging`).

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use regex::Regex;

/// The encoding scheme identified in a decoded payload.
#[derive(Debug, Clone, PartialEq)]
pub enum EncodingScheme {
    Base64,
    UrlEncoding,
    UnicodeHomoglyph,
    Rot13,
    HexSequence,
}

/// A single decoded payload extracted from the prompt.
#[derive(Debug, Clone)]
pub struct DecodedPayload {
    /// The encoding scheme that was detected.
    pub scheme: EncodingScheme,
    /// The decoded plaintext, ready for re-evaluation by `RuleEngine`.
    pub plaintext: String,
    /// Byte offset in the original prompt where the encoded segment was found.
    pub offset: usize,
    /// The depth of recursion this payload was decoded at (1-indexed).
    pub depth: u8,
    /// The raw encoded string that was decoded.
    pub raw_encoded: String,
}

/// Orchestrates the full decoding cascade over a raw prompt string.
///
/// Each decoder in the cascade runs independently; a prompt may yield multiple
/// `DecodedPayload`s if it contains several encoded segments.
pub struct DecoderEngine {
    base64_regex: Regex,
    hex_regex: Regex,
}

impl DecoderEngine {
    /// Create a new `DecoderEngine`.
    pub fn new() -> Self {
        Self {
            // Matches base64 strings of at least 16 chars
            base64_regex: Regex::new(r"(?:[A-Za-z0-9+/]{4}){4,}(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?").unwrap(),
            // Matches hex strings of at least 16 chars
            hex_regex: Regex::new(r"(?i)[a-f0-9]{16,}").unwrap(),
        }
    }

    /// Run all decoders over `prompt` and return every payload found recursively (up to depth 3).
    ///
    /// Returns an empty `Vec` if no encoded segments are detected.
    pub fn decode(&self, prompt: &str) -> Vec<DecodedPayload> {
        self.decode_recursive(prompt, 1)
    }

    fn decode_recursive(&self, text: &str, depth: u8) -> Vec<DecodedPayload> {
        if depth > 3 {
            return Vec::new();
        }

        let mut payloads = Vec::new();

        // 1. URL Encoding (decode whole prompt if contains '%')
        if text.contains('%') {
            if let Ok(decoded) = urlencoding::decode(text) {
                if decoded != text {
                    payloads.push(DecodedPayload {
                        scheme: EncodingScheme::UrlEncoding,
                        plaintext: decoded.into_owned(),
                        offset: 0,
                        depth,
                        raw_encoded: text.to_string(),
                    });
                }
            }
        }

        // 2. Base64
        for cap in self.base64_regex.captures_iter(text) {
            let m = cap.get(0).unwrap();
            let encoded = m.as_str();
            
            if let Ok(decoded_bytes) = BASE64.decode(encoded) {
                if let Ok(s) = String::from_utf8(decoded_bytes) {
                    payloads.push(DecodedPayload {
                        scheme: EncodingScheme::Base64,
                        plaintext: s,
                        offset: m.start(),
                        depth,
                        raw_encoded: encoded.to_string(),
                    });
                }
            }
        }

        // 3. Hex
        for cap in self.hex_regex.captures_iter(text) {
            let m = cap.get(0).unwrap();
            let encoded = m.as_str();
            
            if encoded.len() % 2 == 0 {
                let mut bytes = Vec::with_capacity(encoded.len() / 2);
                let mut valid = true;
                for i in (0..encoded.len()).step_by(2) {
                    if let Ok(b) = u8::from_str_radix(&encoded[i..i+2], 16) {
                        bytes.push(b);
                    } else {
                        valid = false;
                        break;
                    }
                }
                if valid {
                    if let Ok(s) = String::from_utf8(bytes) {
                        payloads.push(DecodedPayload {
                            scheme: EncodingScheme::HexSequence,
                            plaintext: s,
                            offset: m.start(),
                            depth,
                            raw_encoded: encoded.to_string(),
                        });
                    }
                }
            }
        }

        // 4. ROT-13 (decode whole prompt)
        let rot13: String = text.chars().map(|c| {
            match c {
                'a'..='m' | 'A'..='M' => ((c as u8) + 13) as char,
                'n'..='z' | 'N'..='Z' => ((c as u8) - 13) as char,
                _ => c,
            }
        }).collect();
        // Since rot13 affects any english text, it might just produce gibberish.
        // We only append it. (A real implementation would score this output).
        if rot13 != text {
            payloads.push(DecodedPayload {
                scheme: EncodingScheme::Rot13,
                plaintext: rot13,
                offset: 0,
                depth,
                raw_encoded: text.to_string(),
            });
        }

        // Recursion step (only recurse on chunk-based decoders to avoid infinite ROT13 loops)
        let mut nested = Vec::new();
        for p in &payloads {
            if p.scheme == EncodingScheme::Base64 || p.scheme == EncodingScheme::HexSequence {
                let mut inner_payloads = self.decode_recursive(&p.plaintext, depth + 1);
                nested.append(&mut inner_payloads);
            }
        }
        
        payloads.append(&mut nested);
        payloads
    }
}

impl Default for DecoderEngine {
    fn default() -> Self {
        Self::new()
    }
}
