# Basalt

> **Yasal Uyari / Disclaimer:** Resmi bir Minecraft urunu degildir. Mojang veya Microsoft tarafindan onaylanmamistir ve bunlarla iliskili degildir.  
> *NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.*

Windows 10/11 uzerinde **Tauri 2 + TypeScript WebUI** ve **Rust** cekirdegiyle calisan; Fabric Loom uzerinden **Minecraft Java Edition 1.21.4** baslatan masaustu istemci baslaticisi.

## Indirme (GitHub Releases)

- Hazir tek calistirilabilir Windows paketi (`basalt.exe`): [GitHub Releases](https://github.com/MarkesrBetter/Basalt/releases/latest)
- Tanitim sitesi: [site/index.html](./site/index.html)
- Dogrulama (`v0.1.0` SHA-256): `213d6de37c87bfeee122f6572ed61fd33598250f11c69dc30669c98225e1f134`

## Neler Sunar

- **Tek Dosya Masaustu Uygulamasi (`basalt.exe`):** Kurulum sihirbazi gerektirmeden calisir; `client-gradle/` calisma sablonunu ilk calistirmada kullanicinin yerel veri klasorune (`%LOCALAPPDATA%\Basalt\Basalt\data\client-gradle`) acar.
- **Donanima Gore Hiz ve FPS Kalibrasyonu:** `sysinfo` + guvenli bas-siz `wgpu` DX12 yoklamasiyla CPU cekirdegi, RAM ve GPU sinifini okur; `-Xmx`, `G1GC`/`ZGC` ve `options.txt` ayarlarini otomatik kalibre eder.
- **Otomatik Java 21 Kurulumu:** Sistemde Java `>= 21` yoksa uygulama icinden imzali `curl.exe` ile Adoptium Temurin 21 JDK paketini indirir, saf Rust SHA-256 dogrulamasiyla kontrol edip yerel ortama kurar.
- **10 Mineral Temasi ve Tam Ekran Konsol Modu:** 7 koyu ve 3 acik mineral temasi, ozel `#RRGGBB` vurgu kalibrasyonu, 3 kose profili, 5 masaustu duzeni (`Sidebar`, `Topbar`, `CompactDeck`, `RightRail`, `MinimalRail`) ve Steam Deck / kontrolcu odakli tam ekran **Konsol Modu**.
- **Skin ve Dunya Yonetimi:** Yerel `64x64` / `64x32` PNG skin yukleme, `Slim (3 px)` / `Classic (4 px)` kol secimi, yerel `run/saves` dunyalarini yonetme ve tek tikla Quick Play baslatma.

## Kapsam ve Guvenlik Ilkeleri

- **Telifli Dosya Barindirmaz:** Resmi Mojang/Minecraft oyun JAR dosyalari, kaynak paketleri ve `.ogg` arayuz sesleri Git deposuna veya `basalt.exe` ikilisine gomulmez. Ilk oyun baslatmasinda kullanicinin bilgisayarinda resmi Gradle, Fabric ve Mojang kaynak uclarindan yerel onbellege indirilir (`%LOCALAPPDATA%\Basalt\Basalt\data\`).
- **Kimlik Dogrulama Atlama Yok:** Yalnizca yerel tek oyunculu (`--quickPlaySingleplayer`) ve `online-mode=false` yerel gelistirme sunuculari desteklenir. Sunucu yoklayicisi `EncryptionRequest (0x01)` paketi geldiginde baglantiyi keser; Microsoft lisans dogrulamasini atlatmaz.
- **Acik Kaynak Seffafligi:** Release varliklari (`basalt.exe`) Git history icinde degil, **GitHub Releases** uzerinden SHA-256 dogrulama ozetiyle yayinlanir.

## Kaynak Koddan Derleme

Gereksinimler: **Rust (`cargo`)**, **Node.js (`npm`)**, **Java 21 JDK** ve **Windows Edge WebView2 Runtime** (Windows 10/11 ile yerlesik gelir).

```powershell
cd frontend
npm.cmd install
npm.cmd run build
cd ..\client-gradle
.\gradlew.bat build --no-daemon
cd ..
cargo test --workspace
cargo build --release --workspace
.\target\release\basalt.exe
```

## Lisanslar ve Ucuncu Taraf Bilesenler

- Basalt kaynak kodu: [MIT License](./LICENSE)
- Gradle Wrapper (`client-gradle/gradle/wrapper/`): Apache License 2.0
- Masaustu kabugu: Tauri 2 (MIT / Apache-2.0)
- Varsayilan skin sablonu (`assets/default_skin.png`): Depoyu catallamadan / yayinlamadan once kendi orijinal skin dokunuzla degistirebilirsiniz.
