use crate::Data;
use moonbot_db as db;
use once_cell::sync::OnceCell;
use std::sync::RwLock;

// Lightweight caches; safe to be best-effort only
static GLOBAL_CONTEXT_CACHE: OnceCell<RwLock<Option<Vec<String>>>> = OnceCell::new();
static DISPOSITION_CACHE: OnceCell<RwLock<Option<db::Disposition>>> = OnceCell::new();

fn global_ctx_cache() -> &'static RwLock<Option<Vec<String>>> {
	GLOBAL_CONTEXT_CACHE.get_or_init(|| RwLock::new(None))
}
fn disposition_cache() -> &'static RwLock<Option<db::Disposition>> {
	DISPOSITION_CACHE.get_or_init(|| RwLock::new(None))
}

pub async fn invalidate_global_context() { *global_ctx_cache().write().unwrap() = None; }
pub async fn invalidate_disposition() { *disposition_cache().write().unwrap() = None; }

/// Build the base system prompt using global/channel context, bot disposition, and user profile hints.
pub async fn build_system_prompt(data: &Data, user_id: i64, user_name: &str) -> String {
	// Global system context
	let cached_ctx = { global_ctx_cache().read().unwrap().clone() };
	let sys_ctx = if let Some(cached) = cached_ctx {
		cached
	} else {
		let fresh = if let Some(db_ctx) = db::get_global_system_context(data.db).await {
			db_ctx
		} else {
			data.config.openai.auto.system_context.clone()
		};
		*global_ctx_cache().write().unwrap() = Some(fresh.clone());
		fresh
	};

	// Bot disposition
	let cached_disp = { disposition_cache().read().unwrap().clone() };
	let disposition = if let Some(cached) = cached_disp {
		Some(cached)
	} else {
		let fresh = db::get_bot_disposition(data.db).await;
		if let Some(ref d) = fresh { *disposition_cache().write().unwrap() = Some(d.clone()); }
		fresh
	};

	// User profile
	let profile = db::get_user_profile(data.db, user_id).await;

	let mut parts: Vec<String> = Vec::new();
	if !sys_ctx.is_empty() {
		parts.push(format!("### Global context\n{}", sys_ctx.join("\n")));
	}
	if let Some(d) = disposition {
		parts.push(format!("### Bot disposition\nmood='{}' level={} notes={}", d.mood, d.mood_level, d.notes));
	}
	if let Some(p) = profile {
		parts.push(format!(
			"### User profile\nuser_id={} name={}\ntraits={:?}\ntrust_level={}\npreferences={}\nsummary={}",
			user_id,
			user_name,
			p.traits,
			p.trust_level,
			p.preferences,
			p.summary
		));
	}
	// Base operating instruction
	parts.push("### Operating rules\n- Be helpful, concise, and accurate.\n- If unsure, ask a brief clarifying question.\n- Avoid unsolicited replies and refrain from profanity or slurs.".to_string());

	parts.join("\n\n")
}

/// Compute generation parameters adjusted by user preferences and bot mood.
pub async fn compute_generation_params(
	data: &Data,
	user_id: i64,
	base_temperature: f32,
	base_frequency_penalty: f32,
) -> (f32, f32) {
	let mut temp = base_temperature;
	let freq = base_frequency_penalty;

	if let Some(p) = db::get_user_profile(data.db, user_id).await {
		if let Some(style) = p.preferences.get("style").and_then(|v| v.as_str()) {
			match style {
				"concise" => { temp = (temp - 0.1).max(0.0); },
				"detailed" => { temp = (temp + 0.1).min(2.0); },
				_ => {}
			}
		}
	}
	if let Some(d) = db::get_bot_disposition(data.db).await {
		if d.mood_level > 2 { temp = (temp + 0.1).min(2.0); }
		if d.mood_level < -2 { temp = (temp - 0.1).max(0.0); }
	}
	(temp, freq)
}

