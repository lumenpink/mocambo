use sha2::{Digest, Sha256};
use std::sync::Arc;

pub const EMBEDDING_DIM: usize = 512;

pub trait VisionEmbedder: Send + Sync {
    fn embed_image(
        &self,
        image_bytes: &[u8],
    ) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>>;

    fn embed_text(&self, text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>>;
}

#[derive(Clone, Default)]
pub struct DeterministicVisionEmbedder;

impl DeterministicVisionEmbedder {
    pub fn new() -> Self {
        Self
    }

    fn normalize(vec: &mut [f32]) {
        let norm: f32 = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 1e-6 {
            for v in vec.iter_mut() {
                *v /= norm;
            }
        }
    }

    fn extract_visual_tags_from_bytes(image_bytes: &[u8]) -> String {
        let mut tags = Vec::new();

        // 1. Detecção de marcadores JPEG COM (0xFF 0xFE)
        let mut i = 0;
        while i + 4 < image_bytes.len() {
            if image_bytes[i] == 0xFF && image_bytes[i + 1] == 0xFE {
                let len = u16::from_be_bytes([image_bytes[i + 2], image_bytes[i + 3]]) as usize;
                if len >= 2 && i + 2 + len <= image_bytes.len() {
                    let comment_bytes = &image_bytes[i + 4..i + 2 + len];
                    let comment_str = String::from_utf8_lossy(comment_bytes);
                    tags.push(comment_str.to_string());
                    i += 2 + len;
                    continue;
                }
            }
            i += 1;
        }

        // 2. Extração de strings legíveis nos primeiros 4KB (EXIF/XMP/IPTC/nome embutido)
        let probe_len = image_bytes.len().min(4096);
        let probe = &image_bytes[..probe_len];
        let mut current_word = Vec::new();
        for &b in probe {
            if b.is_ascii_alphanumeric() || b == b'_' || b == b'-' {
                current_word.push(b);
            } else {
                if current_word.len() >= 3 {
                    if let Ok(s) = std::str::from_utf8(&current_word) {
                        tags.push(s.to_lowercase());
                    }
                }
                current_word.clear();
            }
        }
        if current_word.len() >= 3 {
            if let Ok(s) = std::str::from_utf8(&current_word) {
                tags.push(s.to_lowercase());
            }
        }

        tags.join(" ")
    }
}

impl VisionEmbedder for DeterministicVisionEmbedder {
    fn embed_image(
        &self,
        image_bytes: &[u8],
    ) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
        let mut vec = vec![0.0f32; EMBEDDING_DIM];

        // Extrai metadados/tags visuais da imagem
        let tags = Self::extract_visual_tags_from_bytes(image_bytes);
        if !tags.is_empty() {
            let text_vec = self.embed_text(&tags)?;
            for i in 0..EMBEDDING_DIM {
                vec[i] += text_vec[i] * 0.85;
            }
        }

        // Extrai assinatura perceptual dos blocos de dados
        for (idx, chunk) in image_bytes.chunks(32).enumerate() {
            let mut hasher = Sha256::new();
            hasher.update((idx as u64).to_le_bytes());
            hasher.update(chunk);
            let hash = hasher.finalize();
            let dim = u16::from_le_bytes([hash[0], hash[1]]) as usize % EMBEDDING_DIM;
            let val = (hash[2] as f32 / 255.0) - 0.5;
            vec[dim] += val * 0.15;
        }

        Self::normalize(&mut vec);
        Ok(vec)
    }

    fn embed_text(&self, text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
        let mut vec = vec![0.0f32; EMBEDDING_DIM];

        for word in text.split_whitespace() {
            let clean: String = word
                .to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if clean.is_empty() {
                continue;
            }

            let mut hasher = Sha256::new();
            hasher.update(clean.as_bytes());
            let hash = hasher.finalize();

            // Espalha energia em 16 dimensões determinísticas por token
            for k in 0..16 {
                let dim =
                    u16::from_le_bytes([hash[k * 2], hash[k * 2 + 1]]) as usize % EMBEDDING_DIM;
                let sign = if hash[k] % 2 == 0 { 1.0f32 } else { -1.0f32 };
                vec[dim] += sign;
            }
        }

        Self::normalize(&mut vec);
        Ok(vec)
    }
}

pub fn create_default_embedder() -> Arc<dyn VisionEmbedder> {
    Arc::new(DeterministicVisionEmbedder::new())
}
