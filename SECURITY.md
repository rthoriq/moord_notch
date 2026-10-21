# Keamanan / Security

Menemukan celah keamanan di NotchWindows? Laporkan secara pribadi lewat tab **Security → Report a
vulnerability** di repo ini, jangan lewat issue publik, supaya bisa diperbaiki sebelum diketahui
banyak orang. Sertakan versi notch, versi Windows, dan langkah untuk memunculkannya.

Found a security issue? Please report it privately via **Security → Report a vulnerability** in this
repository instead of a public issue. Include the NotchWindows version, your Windows version, and
steps to reproduce.

Yang dianggap penting / In scope:

- langkah Administrator untuk pesan masuk (`src/identity.rs`): sertifikat, tanda tangan, paket;
- pemasang dan pencabut (`src/install.rs`);
- data yang keluar dari komputer (lirik dan cuaca, keduanya opsional) dan isi pesan masuk.
