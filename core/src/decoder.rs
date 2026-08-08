//! Encoded-payload detection and decoding engine for `promptify-core`.
//!
//! **Owns**: detecting and decoding encoded attack payloads embedded in prompts
//!           (Base64, URL-encoding, Unicode homoglyphs, ROT-13, hex sequences)
//!           and producing plaintext `DecodedPayload`s for re-evaluation by
//!           `RuleEngine`.
//! **Does not own**: rule evaluation on decoded text (→ `rules`), risk scoring
//!                   (→ `scoring`), or persistence (→ `logging`).

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

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
}

/// Orchestrates the full decoding cascade over a raw prompt string.
///
/// Each decoder in the cascade runs independently; a prompt may yield multiple
/// `DecodedPayload`s if it contains several encoded segments.
pub struct DecoderEngine;

impl DecoderEngine {
    /// Create a new `DecoderEngine`.
    pub fn new() -> Self {
        Self
    }

    /// Run all decoders over `prompt` and return every payload found.
    ///
    /// Returns an empty `Vec` if no encoded segments are detected.
    pub fn decode(&self, prompt: &str) -> Vec<DecodedPayload> {
        let mut payloads = Vec::new();

        // 1. URL Encoding (decode whole prompt if contains '%')
        if prompt.contains('%') {
            if let Ok(decoded) = urlencoding::decode(prompt) {
                if decoded != prompt {
                    payloads.push(DecodedPayload {
                        scheme: EncodingScheme::UrlEncoding,
                        plaintext: decoded.into_owned(),
                        offset: 0,
                    });
                }
            }
        }

        // 2. Base64 & Hex (naive word-based detection for Phase 2)
        for (i, word) in prompt.split_whitespace().enumerate() {
            // Very naive check for Base64 (at least 16 chars to avoid noise)
            if word.len() >= 16 && word.len() % 4 == 0 && word.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=') {
                if let Ok(decoded_bytes) = BASE64.decode(word) {
                    if let Ok(s) = String::from_utf8(decoded_bytes) {
                        if let Some(offset) = prompt.find(word) {
                            payloads.push(DecodedPayload {
                                scheme: EncodingScheme::Base64,
                                plaintext: s,
                                offset,
                            });
                        }
                    }
                }
            }

            // Hex sequence check
            if word.len() >= 16 && word.len() % 2 == 0 && word.chars().all(|c| c.is_ascii_hexdigit()) {
                // Not implemented deeply in phase 2 but stub logic
                let mut bytes = Vec::new();
                let mut valid = true;
                for i in (0..word.len()).step_by(2) {
                    if let Ok(b) = u8::from_str_radix(&word[i..i+2], 16) {
                        bytes.push(b);
                    } else {
                        valid = false;
                        break;
                    }
                }
                if valid {
                    if let Ok(s) = String::from_utf8(bytes) {
                        if let Some(offset) = prompt.find(word) {
                            payloads.push(DecodedPayload {
                                scheme: EncodingScheme::HexSequence,
                                plaintext: s,
                                offset,
                            });
                        }
                    }
                }
            }
        }

        // 3. ROT-13 (decode whole prompt)
        let rot13: String = prompt.chars().map(|c| {
            match c {
                'a'..='m' | 'A'..='M' => ((c as u8) + 13) as char,
                'n'..='z' | 'N'..='Z' => ((c as u8) - 13) as char,
                _ => c,
            }
        }).collect();
        // Since rot13 affects any english text, it might just produce gibberish.
        // We only append it. (A real implementation would score this output).
        if rot13 != prompt {
            payloads.push(DecodedPayload {
                scheme: EncodingScheme::Rot13,
                plaintext: rot13,
                offset: 0,
            });
        }

        payloads
    }
}

impl Default for DecoderEngine {
    fn default() -> Self {
        Self::new()
    }
}
