//! Encoded-payload detection and decoding engine for `promptify-core`.
//!
//! **Owns**: detecting and decoding encoded attack payloads embedded in prompts
//!           (Base64, URL-encoding, Unicode homoglyphs, ROT-13, hex sequences)
//!           and producing plaintext `DecodedPayload`s for re-evaluation by
//!           `RuleEngine`.
//! **Does not own**: rule evaluation on decoded text (→ `rules`), risk scoring
//!                   (→ `scoring`), or persistence (→ `logging`).

/// The encoding scheme identified in a decoded payload.
#[derive(Debug, Clone)]
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
    pub fn decode(&self, _prompt: &str) -> Vec<DecodedPayload> {
        // TODO(Phase 2): implement Base64, URL, homoglyph, ROT-13, hex decoders.
        todo!("Phase 2: implement decoding cascade")
    }
}

impl Default for DecoderEngine {
    fn default() -> Self {
        Self::new()
    }
}
