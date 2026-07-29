//! The text syncparty posts to Discord.
//!
//! Written in the host's own language, because the people reading it are the
//! host's friends rather than the host.

use crate::core::invite::Invite;

/// The "we're live" announcement, including the one-click join link.
pub fn party_ready(invite: &Invite, language: &str) -> String {
    if is_turkish(language) {
        format!(
            "🎬 **Film gecesi hazır!**\n\n\
             **Tek tıkla katıl:** {link}\n\
             **Davet kodu:** `{code}`\n\n\
             **Elle bağlanmak isteyenler için**\n\
             Sunucu: `{address}`\n\
             Oda: `{room}`\n\
             Parola: `{password}`\n\n\
             **İlk kez katılacaklar**\n\
             1. syncparty'yi kurun ve davet bağlantısına tıklayın — gerisini o halleder.\n\
             2. Tailscale hesabınızla giriş yapın; bu bilgisayar sizinle paylaşılmadıysa davet isteyin.\n\n\
             Film dosyası herkeste yerel olarak bulunmalı; dosya internetten yayınlanmıyor.",
            link = invite.deep_link(),
            code = invite.encode(),
            address = invite.server_address(),
            room = invite.room,
            password = invite.password,
        )
    } else {
        format!(
            "🎬 **Movie night is up!**\n\n\
             **One-click join:** {link}\n\
             **Invite code:** `{code}`\n\n\
             **If you would rather connect by hand**\n\
             Server: `{address}`\n\
             Room: `{room}`\n\
             Password: `{password}`\n\n\
             **First time joining**\n\
             1. Install syncparty and open the invite link — it handles the rest.\n\
             2. Sign in with your Tailscale account; ask for an invite if this machine has not been shared with you.\n\n\
             Everyone needs their own copy of the file locally — nothing is streamed.",
            link = invite.deep_link(),
            code = invite.encode(),
            address = invite.server_address(),
            room = invite.room,
            password = invite.password,
        )
    }
}

pub fn party_stopped(language: &str) -> String {
    if is_turkish(language) {
        "🛑 **Film gecesi sunucusu kapatıldı.** Görüşmek üzere!".to_owned()
    } else {
        "🛑 **Movie night server is down.** See you next time!".to_owned()
    }
}

pub fn webhook_test(language: &str) -> String {
    if is_turkish(language) {
        "✅ syncparty Discord bağlantısı çalışıyor.".to_owned()
    } else {
        "✅ syncparty is connected to this channel.".to_owned()
    }
}

/// Matches `tr` and any regional variant such as `tr-TR`.
fn is_turkish(language: &str) -> bool {
    language.split(['-', '_']).next().unwrap_or(language) == "tr"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Invite {
        Invite {
            host: "movie-box.tail1a2b3.ts.net".to_owned(),
            alternate_hosts: vec!["100.79.178.123".to_owned()],
            port: 8999,
            password: "swordfish".to_owned(),
            room: "MovieNight".to_owned(),
        }
    }

    #[test]
    fn recognises_turkish_including_regional_tags() {
        assert!(is_turkish("tr"));
        assert!(is_turkish("tr-TR"));
        assert!(is_turkish("tr_TR"));
        assert!(!is_turkish("en"));
        assert!(!is_turkish("en-GB"));
    }

    #[test]
    fn the_announcement_carries_everything_a_guest_needs() {
        let invite = sample();

        for language in ["tr", "en"] {
            let message = party_ready(&invite, language);

            assert!(message.contains(&invite.deep_link()), "{language}");
            assert!(message.contains(&invite.encode()), "{language}");
            assert!(message.contains(&invite.server_address()), "{language}");
            assert!(message.contains(&invite.room), "{language}");
            assert!(message.contains(&invite.password), "{language}");
        }
    }

    #[test]
    fn falls_back_to_english_for_an_unknown_language() {
        assert_eq!(party_stopped("de"), party_stopped("en"));
    }
}
