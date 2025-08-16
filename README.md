# Sunbot-rs

A simple discord bot using [serenity-rs]
This bot was written to add some fun to my personal discord server, and it not intended to be run in production.

## Features

### User Relationship Tracking

Sunbot now includes sophisticated user relationship tracking that creates individualized pseudo-relationships with each user. This system:

- **Tracks User Sentiment**: Analyzes messages for positive/negative sentiment and adjusts a "temperature" scale from hostile (-1.0) to friendly (1.0)
- **Learns User Interests**: Automatically extracts and stores keywords representing user interests and topics
- **Personalizes Responses**: Includes user relationship context in AI prompts for more personalized interactions
- **Conversation Memory**: Tracks interaction count and relationship notes for each user

#### How It Works

1. **First Interaction**: When a user first interacts with Sunbot, a user record is created with neutral relationship state
2. **Sentiment Analysis**: Each message is analyzed for positive/negative sentiment words, adjusting the user's relationship temperature
3. **Keyword Extraction**: The system identifies topics and interests from user messages and stores them as keywords
4. **Contextual Responses**: When generating AI responses, Sunbot includes the user's relationship context in the prompt

#### Database Schema

The user relationship data is stored in a `user` table with:
- `user_id`: Discord user ID (primary key)
- `username`: Current Discord username
- `display_name`: Discord display name
- `temperature`: Relationship scale (-1.0 to 1.0)
- `keywords`: JSON array of user interests
- `interaction_count`: Total number of interactions
- `last_interaction`: Timestamp of last interaction
- `relationship_notes`: Optional notes about the relationship
- `created_at` / `updated_at`: Timestamps

#### Configuration

Relationship tracking can be configured in your `config.toml`:

```toml
[openai.auto.relationship_tracking]
# Enable/disable user relationship tracking
enabled = true
# Maximum number of keywords to store per user
max_keywords_per_user = 20
# How much to adjust temperature per positive/negative sentiment
temperature_adjustment = 0.1
# Whether to include relationship context in AI prompts
include_context_in_prompts = true
```

#### Example Relationship Context

When enabled, Sunbot will include context like this in AI prompts:

```
User relationship context for alice: Relationship status: friendly. User interests: gaming, python, music. Total interactions: 15
```

This allows the AI to provide more personalized and contextually appropriate responses based on the established relationship and known interests.
