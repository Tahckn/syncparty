//! Turning a party into one string a guest can act on.
//!
//! Everything a guest needs — address, port, password, room — travels as a
//! single token, so nobody has to copy four values out of a chat message and
//! retype them into a connection dialog. The same token doubles as a
//! `syncparty://` link, which opens the app already filled in.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::core::error::{Result, SyncPartyError};

/// URI scheme registered with the OS for one-click joining.
pub const DEEP_LINK_SCHEME: &str = "syncparty";

/// Token prefix. Versioned so a future format can be told apart from this one
/// instead of failing with a confusing parse error.
const TOKEN_PREFIX: &str = "SP1.";

/// A party, in the form a guest receives it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Invite {
    /// Tailscale address or MagicDNS name of the host.
    pub host: String,
    pub port: u16,
    pub password: String,
    pub room: String,
}

/// The on-the-wire payload. Keys are abbreviated because the whole thing ends
/// up base64-encoded in a chat message, and a shorter token is a friendlier
/// one to paste.
#[derive(Serialize, Deserialize)]
struct Payload {
    v: u8,
    h: String,
    p: u16,
    pw: String,
    r: String,
}

impl Invite {
    /// Encodes the invite as a `SP1.…` token.
    pub fn encode(&self) -> String {
        let payload = Payload {
            v: 1,
            h: self.host.clone(),
            p: self.port,
            pw: self.password.clone(),
            r: self.room.clone(),
        };

        // Serialising a struct we own cannot fail.
        let json = serde_json::to_vec(&payload).unwrap_or_default();
        format!("{TOKEN_PREFIX}{}", URL_SAFE_NO_PAD.encode(json))
    }

    /// Parses a token, a deep link, or a chat message containing either.
    ///
    /// Guests paste whatever they have — the bare token, the full link, the
    /// line of surrounding text — so this accepts all of it rather than
    /// asking them to trim it first.
    pub fn decode(input: &str) -> Result<Self> {
        let token = extract_token(input)
            .ok_or_else(|| SyncPartyError::InvalidInvite("no invite code found".to_owned()))?;

        let encoded = token.trim_start_matches(TOKEN_PREFIX);
        let json = URL_SAFE_NO_PAD.decode(encoded).map_err(|_| {
            SyncPartyError::InvalidInvite("the code is not valid base64".to_owned())
        })?;

        let payload: Payload = serde_json::from_slice(&json)
            .map_err(|_| SyncPartyError::InvalidInvite("the code is damaged".to_owned()))?;

        if payload.v != 1 {
            return Err(SyncPartyError::InvalidInvite(format!(
                "this code was made by a newer version of syncparty (format {})",
                payload.v
            )));
        }

        if payload.h.is_empty() || payload.r.is_empty() || payload.p == 0 {
            return Err(SyncPartyError::InvalidInvite(
                "the code is missing an address, port or room".to_owned(),
            ));
        }

        Ok(Self {
            host: payload.h,
            port: payload.p,
            password: payload.pw,
            room: payload.r,
        })
    }

    /// The clickable form: `syncparty://join/SP1.…`.
    pub fn deep_link(&self) -> String {
        format!("{DEEP_LINK_SCHEME}://join/{}", self.encode())
    }

    /// `host:port`, the way the Syncplay client wants it.
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Finds a `SP1.…` token anywhere in `input`.
fn extract_token(input: &str) -> Option<&str> {
    let start = input.find(TOKEN_PREFIX)?;
    let rest = &input[start..];

    // base64url plus the prefix's dot; anything else ends the token.
    let end = rest
        .char_indices()
        .position(|(index, character)| {
            index >= TOKEN_PREFIX.len()
                && !(character.is_ascii_alphanumeric() || character == '-' || character == '_')
        })
        .unwrap_or(rest.len());

    Some(&rest[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Invite {
        Invite {
            host: "movie-box.tail1a2b3.ts.net".to_owned(),
            port: 8999,
            password: "swordfish".to_owned(),
            room: "MovieNight".to_owned(),
        }
    }

    #[test]
    fn survives_a_round_trip() {
        let invite = sample();

        assert_eq!(Invite::decode(&invite.encode()).expect("decode"), invite);
    }

    #[test]
    fn tokens_are_url_safe_so_they_survive_chat_apps() {
        let invite = Invite {
            host: "host~with/odd+chars".to_owned(),
            ..sample()
        };

        let token = invite.encode();
        assert!(token.starts_with(TOKEN_PREFIX));
        assert!(token
            .trim_start_matches(TOKEN_PREFIX)
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
        assert_eq!(Invite::decode(&token).expect("decode"), invite);
    }

    #[test]
    fn decodes_a_deep_link() {
        let invite = sample();

        assert_eq!(Invite::decode(&invite.deep_link()).expect("decode"), invite);
    }

    #[test]
    fn digs_the_token_out_of_a_chat_message() {
        let invite = sample();
        let message = format!(
            "hey everyone, film starts at 9 — join with {} (bring snacks)",
            invite.encode()
        );

        assert_eq!(Invite::decode(&message).expect("decode"), invite);
    }

    #[test]
    fn deep_links_use_the_registered_scheme() {
        assert!(sample().deep_link().starts_with("syncparty://join/SP1."));
    }

    #[test]
    fn rejects_input_with_no_code_in_it() {
        let error = Invite::decode("good evening").expect_err("no code");

        assert_eq!(error.kind(), "invalid_invite");
    }

    #[test]
    fn rejects_a_corrupted_code() {
        let error = Invite::decode("SP1.notrealbase64payload").expect_err("damaged");

        assert_eq!(error.kind(), "invalid_invite");
    }

    #[test]
    fn rejects_a_future_format_with_a_useful_message() {
        let payload = serde_json::to_vec(&Payload {
            v: 99,
            h: "host".to_owned(),
            p: 8999,
            pw: String::new(),
            r: "room".to_owned(),
        })
        .expect("encode");
        let token = format!("{TOKEN_PREFIX}{}", URL_SAFE_NO_PAD.encode(payload));

        let error = Invite::decode(&token).expect_err("future format");
        assert!(error.to_string().contains("newer version"));
    }

    #[test]
    fn rejects_a_code_missing_its_address() {
        let payload = serde_json::to_vec(&Payload {
            v: 1,
            h: String::new(),
            p: 8999,
            pw: String::new(),
            r: "room".to_owned(),
        })
        .expect("encode");
        let token = format!("{TOKEN_PREFIX}{}", URL_SAFE_NO_PAD.encode(payload));

        assert!(Invite::decode(&token).is_err());
    }

    #[test]
    fn an_empty_password_round_trips() {
        let invite = Invite {
            password: String::new(),
            ..sample()
        };

        assert_eq!(Invite::decode(&invite.encode()).expect("decode"), invite);
    }

    #[test]
    fn formats_the_address_the_way_the_client_expects() {
        assert_eq!(sample().server_address(), "movie-box.tail1a2b3.ts.net:8999");
    }
}
