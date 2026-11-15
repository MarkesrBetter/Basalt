# Basalt — Ekran ve Metin Envanteri (`content-map.md`)

## 1. Ürün Bağlamı ve Okuma Yoğunluğu
- **Ürün Türü:** Native Windows 10/11 masaüstü başlatıcısı (`Tauri 2 + TypeScript WebUI + Rust`). Resmi Minecraft Java Edition (Vanilla `1.21.4` – `1.16.5`) sürümlerini mod yükleyici olmadan doğrudan `java.exe` üzerinden başlatır.
- **Birincil 3 Kullanıcı Eylemi:**
  1. Seçili sürüm/dünya/yerel sunucu ile oyunu başlatmak (`Launch Minecraft` / `Minecraft'ı Başlat`).
  2. Tek oyunculu dünyaları (`saves/`) filtrelemek, sıralamak, Quick Play hedefi seçmek veya kalıcı silmek.
  3. Çevrimdışı kullanıcı adını, 64×64 PNG skin paketini, donanıma kalibre `options.txt`/JVM belleğini ve Java 21 kurulumunu yönetmek.
- **En Riskli Hata Akışları:**
  - Bilgisayarda Java 21 bulunmaması (`play.javaSetupTitle` -> tek tıkla SHA-256 doğrulamalı Eclipse Temurin 21 kurulumu).
  - Kayıtlı bir dünyanın diskten kalıcı olarak silinmesi (`#delete-world-dialog` -> nesne adı, boyut, dizin yolu ve geri alınamazlık uyarısı).
  - Çevrimiçi (`EncryptionRequest` isteyen) bir sunucuya çevrimdışı bağlanma girişimi (`servers.probeAuth` -> kimlik doğrulamanın asla atlatılmadığının açık bildirimi).

## 2. Ekran Envanteri ve Semantik Başlık Hiyerarşisi

| Ekran / Görünüm | DOM Semantiği | Anahtar Metin Grupları | Okuma Yoğunluğu |
| :--- | :--- | :--- | :--- |
| **01 / Play (`#view-play`)** | `<section aria-labelledby="play-heading">` -> `<h1>` (`play.heading`), `<h2>` (`play-world-title`, `play.recentTitle`, `play.specsTitle`) | Başlatma hedefi, sürüm seçimi (`1.21.4`–`1.16.5`), hızlı giriş, çevrimdışı oyuncu adı, Java 21 kurulum kartı, hızlı geçiş listesi, çalışma zamanı günlüğü (`role="log"`) | Kısa kontrol + durum özeti |
| **02 / Saved Worlds (`#view-worlds`)** | `<section aria-labelledby="worlds-heading">` -> `<h1>` (`worlds.heading`), arama/sıralama araç çubuğu (`.worlds-toolbar`), `<div role="list">` (`#worlds-list`) | Dünya filtreleme (`worlds.filterLabel`), sıralama (`worlds.sortLabel`), toplam/filtrelenen kayıt sayısı (`worlds.countLabel` / `worlds.filteredCountLabel`), boş durum / filtre sonucu yok durumu (`#worlds-state-banner`), satır eylemleri | Yoğun veri listesi |
| **03 / Profile & Skin (`#view-profile`)** | `<section aria-labelledby="profile-heading">` -> `<h1>` (`profile.heading`), `<h2>` (`profile.identityTitle`, `profile.sessionCardTitle`, `skin.sectionTitle`) | Çevrimdışı kullanıcı adı formu (`aria-describedby="profile-username-error"`), UUID v3 önizlemesi, 64×64 PNG skin önizleme kanvası (`Ön / Arka / Doku`), Slim/Klasik kol modeli, `<details>` görünüm çekmecesi | Form + görsel önizleme |
| **04 / Settings (`#view-system`)** | `<section aria-labelledby="system-heading">` -> `<h1>` (`system.heading`), `<h2>` (`system.generalTitle`, `system.gfxTitle`, `turbo.pingTitle`, `servers.title`) | Arayüz dili (`EN / TR / DE`), Minecraft sürümü, `options.txt` & JVM (`-Xmx`, `G1GC`/`ZGC`, chunk/FOV), ağ/gecikme tanılaması, Discord Rich Presence, yerel sunucu SLP/el sıkışma listesi, `<details>` donanım çekmecesi | Yoğun ayar ve tablo yüzeyi |
| **Delete World Modal (`#delete-world-dialog`)** | `<dialog aria-labelledby="delete-dialog-title" aria-describedby="delete-dialog-desc">` -> `<h2>` | Kalıcı işlem üst başlığı, silinecek dünya adı + boyutu + tam dizin yolu, `Vazgeç` ve `Kalıcı olarak sil` eylemleri | Kritik karar / yıkıcı onay |
| **Product Website (`site/index.html`)** | `<header>`, `<main>`, `<h1>` (`hero.h1`), `<h2>` (`feat.h2`, `specs.h2`, `dl.h2`, `faq.h2`), `<footer>` | Canlı uygulama önizlemesi, 6 özellik kartı, teknik mimari ve donanım profilleri tablosu, `basalt.exe` doğrudan indirme + SHA-256 doğrulama kutusu, SSS | Editoryal tanıtım + teknik doğrulama |
