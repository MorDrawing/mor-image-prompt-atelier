# Maintainer: local
# Local desktop package for Mor Image Prompt Atelier.
# Build & install from the project root:
#   makepkg -si

pkgname=mor-image-prompt-atelier
pkgver=0.1.0
pkgrel=1
pkgdesc="Desktop atelier for crafting and collecting image prompts"
arch=('x86_64')
url="https://local/mor-image-prompt-atelier"
license=('LicenseRef-Proprietary')
depends=(
  'gtk3'
  'webkit2gtk-4.1'
  'libayatana-appindicator'
  'cairo'
  'pango'
  'gdk-pixbuf2'
  'libsoup3'
)
makedepends=(
  'cargo'
  'rust'
  'pkgconf'
  'librsvg'
)
options=('!lto' '!debug')
source=()
sha256sums=()

prepare() {
  cd "${startdir}"
  export RUSTUP_TOOLCHAIN=stable
  cargo fetch --locked --target "$(rustc -vV | sed -n 's/host: //p')"
}

build() {
  cd "${startdir}"
  export RUSTUP_TOOLCHAIN=stable
  export CARGO_TARGET_DIR="${startdir}/target"
  # Avoid embedding absolute paths / keep rebuilds predictable.
  cargo build --frozen --release --bin mor_image_prompt_atelier
}

package() {
  cd "${startdir}"

  install -Dm755 "target/release/mor_image_prompt_atelier" \
    "${pkgdir}/usr/bin/mor-image-prompt-atelier"

  # Dioxus asset root layout: /usr/lib/<name>/assets/ (also embedded in binary).
  install -Dm644 "assets/style.css" \
    "${pkgdir}/usr/lib/${pkgname}/assets/style.css"

  install -Dm644 "packaging/mor-image-prompt-atelier.desktop" \
    "${pkgdir}/usr/share/applications/mor-image-prompt-atelier.desktop"

  install -Dm644 "assets/icons/mor-image-prompt-atelier.svg" \
    "${pkgdir}/usr/share/icons/hicolor/scalable/apps/mor-image-prompt-atelier.svg"

  local size
  for size in 16 22 24 32 48 64 128 256 512; do
    install -Dm644 "assets/icons/hicolor/${size}x${size}/apps/mor-image-prompt-atelier.png" \
      "${pkgdir}/usr/share/icons/hicolor/${size}x${size}/apps/mor-image-prompt-atelier.png"
  done
}
