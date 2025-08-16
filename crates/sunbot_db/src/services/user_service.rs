use sea_orm::prelude::*;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use crate::entities::{prelude::*, user::{self, Model as UserModel}};

pub struct UserService;

impl UserService {
    /// Get or create a user by Discord user ID
    pub async fn get_or_create_user(
        db: &DatabaseConnection,
        user_id: i64,
        username: String,
        display_name: Option<String>,
    ) -> Result<UserModel, DbErr> {
        // Try to find existing user
        if let Some(user) = User::find_by_id(user_id).one(db).await? {
            // Update username and display name if they've changed
            if user.username != username || user.display_name != display_name {
                let mut user_active: user::ActiveModel = user.clone().into();
                user_active.username = Set(username);
                user_active.display_name = Set(display_name);
                user_active.updated_at = Set(chrono::Utc::now());
                
                return Ok(user_active.update(db).await?);
            }
            return Ok(user);
        }

        // Create new user
        let new_user = UserModel::new(user_id, username, display_name);
        Ok(new_user.insert(db).await?)
    }

    /// Update user relationship state after an interaction
    pub async fn update_user_interaction(
        db: &DatabaseConnection,
        user_id: i64,
        temperature_delta: Option<f32>,
        new_keywords: Option<Vec<String>>,
        notes: Option<String>,
    ) -> Result<UserModel, DbErr> {
        let user = User::find_by_id(user_id)
            .one(db)
            .await?
            .ok_or(DbErr::RecordNotFound("User not found".to_string()))?;

        let updated_user = user.update_interaction(temperature_delta, new_keywords, notes);
        Ok(updated_user.update(db).await?)
    }

    /// Get user relationship context for AI prompting
    pub async fn get_user_context(
        db: &DatabaseConnection,
        user_id: i64,
    ) -> Result<Option<String>, DbErr> {
        if let Some(user) = User::find_by_id(user_id).one(db).await? {
            Ok(Some(user.get_relationship_context()))
        } else {
            Ok(None)
        }
    }

    /// Analyze message content and extract potential keywords/sentiment
    pub fn analyze_message_content(content: &str) -> (Option<f32>, Option<Vec<String>>) {
        let content_lower = content.to_lowercase();
        
        // Simple sentiment analysis based on common words
        let positive_words = [
            "love", "like", "enjoy", "awesome", "great", "good", "nice", "cool", 
            "amazing", "wonderful", "fantastic", "excellent", "thanks", "thank you",
            "appreciate", "happy", "glad", "pleased", "excited"
        ];
        
        let negative_words = [
            "hate", "dislike", "terrible", "awful", "bad", "worst", "horrible",
            "stupid", "dumb", "annoying", "angry", "mad", "frustrated", "upset",
            "disappointed", "disgusted"
        ];

        let mut positive_count = 0;
        let mut negative_count = 0;
        
        for word in positive_words.iter() {
            if content_lower.contains(word) {
                positive_count += 1;
            }
        }
        
        for word in negative_words.iter() {
            if content_lower.contains(word) {
                negative_count += 1;
            }
        }

        // Calculate temperature delta
        let temperature_delta = if positive_count > 0 || negative_count > 0 {
            let delta = (positive_count as f32 - negative_count as f32) * 0.1;
            Some(delta.clamp(-0.5, 0.5))
        } else {
            None
        };

        // Extract potential keywords (simple approach - look for capitalized words or common topics)
        let keywords = Self::extract_keywords(&content);

        (temperature_delta, keywords)
    }

    /// Extract keywords from message content
    fn extract_keywords(content: &str) -> Option<Vec<String>> {
        let words: Vec<&str> = content.split_whitespace().collect();
        let mut keywords = Vec::new();
        
        // Common topic keywords to look for
        let topic_keywords = [
            "gaming", "games", "music", "movies", "books", "programming", "coding",
            "art", "drawing", "cooking", "food", "sports", "travel", "photography",
            "anime", "manga", "streaming", "twitch", "youtube", "discord", "reddit",
            "python", "rust", "javascript", "react", "nodejs", "ai", "ml", "crypto"
        ];

        for word in words {
            let word_clean = word.to_lowercase()
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_string();
                
            // Look for topic keywords
            if topic_keywords.contains(&word_clean.as_str()) && !keywords.contains(&word_clean) {
                keywords.push(word_clean);
            }
            
            // Look for capitalized words that might be proper nouns (but not at start of sentence)
            if word.len() > 3 && word.chars().next().unwrap().is_uppercase() {
                if let Some(first_char) = content.chars().next() {
                    if word.as_ptr() != content.as_ptr() || !first_char.is_uppercase() {
                        let clean_word = word.trim_matches(|c: char| !c.is_alphanumeric()).to_string();
                        if clean_word.len() > 3 && !keywords.contains(&clean_word.to_lowercase()) {
                            keywords.push(clean_word.to_lowercase());
                        }
                    }
                }
            }
        }

        if keywords.is_empty() {
            None
        } else {
            Some(keywords)
        }
    }
}