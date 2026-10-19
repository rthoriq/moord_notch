# Notch Windows

Notch pribadi di tengah atas layar laptop, dibuat sendiri dengan Rust + `windows-rs`
(crate resmi Microsoft). Semua berjalan lokal: tidak ada akun atau telemetri, dan satu-satunya
koneksi internet adalah fitur lirik yang mati secara default (lihat Tahap 4). Kabar pesan masuk
dibaca dari notifikasi Windows di laptop sendiri, tanpa internet.

## Tahap 1: dasar notch

- Pil hitam di tengah atas layar, menyatu dengan tepi layar seperti notch MacBook.
- Arahkan kursor ke pil (atau klik) → mengembang jadi panel. Kursor keluar → menutup lagi.
  Animasinya memakai pegas sehingga sedikit memantul.
- Maskot sepasang mata yang sesekali berkedip.
- Mengecil jadi garis tipis saat jendela aktif di-maximize, supaya tidak menutupi tab browser.
- Sembunyi otomatis saat ada aplikasi full-screen (game, video, presentasi).
- Ikon tray: tampilkan/sembunyikan notch, "Mulai bersama Windows", dan keluar.
- Klik di luar bentuk notch tembus ke jendela di bawahnya.

## Tahap 2: musik dan kabar singkat

- **Musik/video**: membaca apa pun yang muncul di kontrol media Windows (Spotify, YouTube di
  browser, Media Player, dll). Saat diputar, pil menampilkan sampul kecil dan gelombang yang
  diwarnai sesuai sampul. Panel menampilkan sampul, judul, artis, progres, serta tombol
  sebelumnya / putar-jeda / berikutnya.
- **Volume**: HUD berisi ikon, bilah, dan persen setiap kali volume atau mute berubah.
  HUD volume bawaan Windows 11 tetap muncul, karena tidak bisa dimatikan secara resmi.
- **Daya**: kabar saat charger dicolok atau dicabut, dan peringatan saat baterai turun ke 20% dan 10%.
- **Bluetooth**: kabar saat perangkat yang sudah dipasangkan tersambung atau terputus, termasuk
  persen baterainya bila perangkat melaporkannya.

## Tahap 3: widget di panel

Panel punya lima halaman: **Beranda** (jam, atau pemutar musik kalau ada media), **Shelf**,
**Pomodoro**, **Minum air**, dan **Tugas**. Pindah halaman dengan roda mouse / geser dua jari,
panah di kiri-kanan panel, atau klik titik di bawah panel. Halaman terakhir diingat.

- **Shelf**: seret file atau folder ke notch, maka panel terbuka di halaman Shelf dan file
  tersimpan di sana. Yang disimpan hanya lokasinya, bukan salinan file. Klik dua kali untuk
  membuka, klik kanan untuk menampilkannya di File Explorer, seret keluar untuk menyalin atau
  memindahkannya ke aplikasi lain. File yang dipindahkan otomatis hilang dari shelf.
- **Pomodoro**: fokus 25 menit lalu istirahat 5 menit yang langsung berjalan. Klik cincin untuk
  mulai/jeda. Selama berjalan, pil menampilkan cincin progres dan sisa menit. Saat waktu habis
  muncul kabar singkat disertai bunyi notifikasi Windows.
- **Minum air**: target 2 L per hari, satu klik "+ Minum" = 250 ml, ada grafik tujuh hari terakhir.
  Pengingat muncul satu jam setelah minum terakhir, lalu tiap 30 menit selama belum minum,
  hanya pukul 07.00–22.00 dan selama target belum tercapai.
- **Tugas**: klik kotak "Tambah tugas" lalu ketik. Enter menyimpan dan siap untuk tugas berikutnya,
  Esc membatalkan, Ctrl+V menempel. Klik tugas untuk mencentang. Daftar panjang digulir dengan
  roda mouse.

Mengklik kabar singkat (misal pengingat minum) membuka halaman yang bersangkutan.

Data disimpan sebagai file teks biasa di `%APPDATA%\NotchWindows`: `shelf.txt`, `todo.txt`,
`water.txt`, dan `settings.txt`. Semuanya bisa dibaca dan diedit dengan Notepad.

## Tahap 4: privasi, lirik, sistem, dan tema

- **Indikator kamera/mikrofon**: titik hijau (kamera) atau oranye (mikrofon) di ujung kanan pil
  selama ada aplikasi yang memakainya, plus kabar singkat "Kamera sedang dipakai · Zoom" saat
  aplikasi baru mulai memakainya. Mikrofon dibaca dari sesi rekam Windows (Core Audio), kamera
  dari pemantau sensor Media Foundation; keduanya mengabari notch tanpa polling. Selama indikator
  menyala, notch tidak menciut jadi garis tipis supaya titiknya tetap terlihat.
- **Lirik (opsional)**: nyalakan lewat menu tray "Lirik lagu (memakai internet)". Lirik
  tersinkron diambil dari [LRCLIB](https://lrclib.net) lewat HTTPS. Yang dikirim hanya judul,
  artis, album, dan durasi lagu yang sedang diputar; hasilnya hanya disimpan di memori. Selama
  lagu diputar, pil menampilkan baris yang sedang dinyanyikan dan lebarnya menyesuaikan panjang
  baris; baris yang terlalu panjang dibagi jadi dua baris. Halaman musik juga menampilkan baris
  lirik (sampai dua baris).
- **Sistem**: halaman keenam berisi beban CPU, GPU, RAM, dan disk C: (warnanya berubah oranye di
  atas 60% dan merah di atas 85%), kecepatan jaringan, lama laptop menyala, dan aplikasi yang
  sedang memakai kamera/mikrofon. CPU dan GPU memakai angka yang sama dengan Task Manager.
  Hanya diukur selama halaman ini terbuka.
- **Tema**: pilih di menu tray → Tema: Hitam (bawaan), Grafit, Biru malam, Anggur, Kaca, atau Terang.
  Kaca membuat notch setengah tembus pandang dengan garis tepi tipis (tanpa blur, karena blur
  di Windows hanya bisa lewat API yang tidak resmi).

## Perbaikan setelah Tahap 4

Semua pengaturan ada di menu tray → **Pengaturan**.

- **Suara notifikasi**: bunyi bawaan Windows untuk pengingat minum, pomodoro selesai, charger,
  Bluetooth, baterai lemah, kamera/mikrofon, dan pesan masuk. Diam otomatis saat full-screen,
  presentasi, atau Focus/Jangan Ganggu. Volume, kecerahan, dan Caps Lock sengaja tanpa bunyi.
- **Lock screen**: notch tetap tampil saat laptop dikunci (Win+L), tapi hanya jam dan kontrol
  musik; halaman lain tidak bisa dibuka, dan kabar pesan hanya berbunyi "Pesan baru" tanpa nama
  pengirim. Layar isi PIN/kata sandi adalah desktop aman Windows, jadi di sana tidak ada aplikasi
  yang bisa tampil.
- **Caps Lock / Num Lock**: kabar singkat "Nyala"/"Mati". Notch hanya memperhatikan dua tombol
  ini; tombol lain langsung diabaikan dan tidak pernah disimpan.
- **Kecerahan layar**: kabar berisi bilah dan persen setiap kecerahan berubah.
- **Pesan masuk** (WhatsApp, atau semua aplikasi chat): kabar berisi logo aplikasi, pengirim,
  dan potongan pesan. Klik kabarnya untuk membuka aplikasinya. "Sembunyikan isi pesan" hanya
  menampilkan pengirim. Windows hanya mengizinkan aplikasi berpaket membaca notifikasi, jadi
  fitur ini perlu dipasang sekali (lihat di bawah).

### Memasang kabar pesan masuk

1. Build versi rilis: `cargo build --release`.
2. Buka PowerShell dengan **Run as Administrator**, masuk ke folder proyek, lalu jalankan:

   ```bash
   powershell -ExecutionPolicy Bypass -File packaging\install.ps1
   ```

3. Kalau Windows bertanya izin membaca notifikasi, pilih izinkan.
4. Menu tray → Pengaturan → Pesan masuk → pilih "Hanya WhatsApp" atau "Semua aplikasi chat".

Skrip ini membuat sertifikat pribadi yang hanya ada di laptop ini, menandatangani paket kecil
berisi manifest dan logo (`packaging\package`), memercayai sertifikatnya, lalu **menghapus kunci
privatnya** supaya sertifikat itu tidak bisa dipakai menandatangani apa pun lagi. notch.exe tetap
di `target\release`; paket hanya memberinya identitas. Tidak ada yang diunduh. Ulangi skrip ini
hanya kalau isi folder `packaging` berubah; build ulang exe biasa tidak perlu. Untuk mencabut:
`packaging\uninstall.ps1` (juga sebagai Administrator).

Kalau izin pernah ditolak, nyalakan lagi di Pengaturan Windows → Privasi & keamanan →
Notifikasi → izinkan aplikasi mengakses notifikasi.

## Build dan jalankan

Butuh Rust (rustup) dan Visual Studio Build Tools dengan workload C++.

```bash
cargo run
```

Build debug membuka jendela konsol berisi log. Untuk versi harian yang ringan tanpa konsol:

```bash
cargo build --release
```

Hasilnya `target\release\notch.exe`. `build.rs` menanam manifest Windows (`packaging\notch.exe.manifest`)
ke exe; isinya penghubung ke paket untuk kabar pesan masuk, dan tidak berpengaruh apa-apa
selama paketnya belum dipasang. Aktifkan "Mulai bersama Windows" dari menu tray
setelah exe berada di lokasi permanennya, karena yang disimpan adalah path exe tersebut.
Keluarkan notch lewat menu tray sebelum build ulang, karena exe yang sedang berjalan terkunci.

`cargo test` merender semua bentuk notch ke `target\preview\*.bmp` untuk dicek tanpa menjalankan aplikasi.

## Struktur kode

| File | Isi |
|---|---|
| `src/main.rs` | Titik masuk, DPI awareness, cegah dua salinan berjalan |
| `src/app.rs` | Jendela, input mouse/keyboard, timer, loop pesan, perpindahan bentuk dan halaman |
| `src/render/mod.rs` | Gambar notch dengan Direct2D (software) ke layered window, alat gambar dasar, ikon tray |
| `src/render/pages.rs` | Tata letak pil, HUD, dan tiap halaman panel; area klik didaftarkan sambil menggambar |
| `src/widgets/` | Data dan aturan shelf, pomodoro, minum air, dan tugas (diuji dengan `cargo test`) |
| `src/store.rs` | Simpan/muat file teks di `%APPDATA%\NotchWindows` |
| `src/dragdrop.rs` | Seret file masuk/keluar, buka file, tampilkan di Explorer |
| `src/thumbnail.rs` | Thumbnail file dari Windows, di thread terpisah |
| `src/anim.rs` | Animasi pegas |
| `src/desktop.rs` | Deteksi full-screen dan jendela maximize lewat event Windows |
| `src/services.rs` | Thread latar untuk layanan WinRT/COM, kabar ke jendela lewat channel |
| `src/media.rs` | Sesi media Windows (SMTC), sampul album, perintah putar |
| `src/audio.rs` | Perubahan volume perangkat output default |
| `src/power.rs` | Charger, persen baterai, dan kecerahan layar |
| `src/bluetooth.rs` | Perangkat Bluetooth tersambung/terputus dan baterainya |
| `src/privacy.rs` | Aplikasi yang sedang memakai kamera (Media Foundation) dan mikrofon (Core Audio) |
| `src/messages.rs` | Pesan masuk dari notifikasi aplikasi chat (butuh identitas paket) |
| `src/sound.rs` | Bunyi notifikasi bawaan Windows, diam saat full-screen atau Jangan Ganggu |
| `src/lyrics.rs` | Lirik tersinkron dari LRCLIB (opsional), pembaca format LRC |
| `src/sysinfo.rs` | CPU, GPU, RAM, disk, dan jaringan |
| `src/settings.rs` | Pengaturan (suara, lirik, kabar, pesan, tema) |
| `src/tray.rs` | Ikon dan menu tray |
| `src/autostart.rs` | Kunci `Run` di registry pengguna (tanpa admin) |
| `src/clock.rs` | Jam, tanggal, durasi, dan lama menyala berbahasa Indonesia |
| `build.rs`, `packaging/` | Manifest exe, paket sparse, dan skrip pasang/cabut kabar pesan |

## Target performa

RAM saat diam di bawah 40 MB, CPU saat diam sekitar 0%, terbuka kurang dari 1 detik.
Notch hanya menggambar saat ada yang bergerak; selebihnya proses tidur menunggu event.
Saat musik diputar, gelombang di pil digambar ulang sekitar 12 kali per detik, tapi hanya
kotak kecil di sekitar gelombangnya (sekitar 10 ms CPU per detik). Kedipan mata maskot juga
hanya menggambar ulang area matanya.

Build debug mencatat tiap menit jumlah frame animasi, callback hook, dan event layanan, untuk
memeriksa kalau ada yang membangunkan proses terlalu sering.
