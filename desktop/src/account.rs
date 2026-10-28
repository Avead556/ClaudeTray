use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub email: String,
    pub name: String,
    pub organization: String,
    pub plan: String,
    pub oauth_account: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
}

impl Account {
    pub fn from_oauth_account(json: &Value) -> Option<Self> {
        let account_uuid = json["accountUuid"].as_str()?;
        let organization_uuid = json["organizationUuid"].as_str().unwrap_or_default();
        let email = json["emailAddress"].as_str().unwrap_or("—").to_owned();
        let mut organization = json["organizationName"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        if organization == format!("{email}'s Organization") {
            organization.clear();
        }
        let name = json["displayName"].as_str().or(json["fullName"].as_str());
        Some(Self {
            id: format!("{account_uuid}:{organization_uuid}"),
            name: name.unwrap_or_default().to_owned(),
            plan: plan(
                json["organizationType"].as_str(),
                json["organizationRateLimitTier"].as_str(),
            ),
            email,
            organization,
            oauth_account: json.clone(),
            nickname: None,
        })
    }

    pub fn title(&self) -> &str {
        self.nickname.as_deref().unwrap_or(&self.email)
    }
}

fn plan(kind: Option<&str>, tier: Option<&str>) -> String {
    let Some(kind) = kind.and_then(|kind| kind.strip_prefix("claude_")) else {
        return String::new();
    };
    let mut chars = kind.chars();
    let mut plan: String = chars
        .next()
        .into_iter()
        .flat_map(char::to_uppercase)
        .chain(chars)
        .collect();
    let multiplier = tier
        .and_then(|tier| tier.rsplit('_').next())
        .filter(|multiplier| {
            multiplier.strip_suffix('x').is_some_and(|digits| {
                !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
            })
        });
    if let Some(multiplier) = multiplier {
        plan.push(' ');
        plan.push_str(multiplier);
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn formats_plans() {
        assert_eq!(
            plan(Some("claude_max"), Some("default_claude_max_5x")),
            "Max 5x"
        );
        assert_eq!(plan(Some("claude_team"), None), "Team");
        assert_eq!(plan(Some("claude_pro"), Some("default_x")), "Pro");
        assert_eq!(plan(None, None), "");
    }

    #[test]
    fn hides_personal_organization() {
        let account = Account::from_oauth_account(&json!({
            "accountUuid": "a", "organizationUuid": "o",
            "emailAddress": "me@example.com", "organizationName": "me@example.com's Organization"
        }))
        .unwrap();
        assert_eq!(account.id, "a:o");
        assert!(account.organization.is_empty());
    }
}
