#!/usr/bin/env bash
set -e

BINARY_NAME="nsr-ssh"
INSTALL_BIN="${HOME}/.local/bin"
INSTALL_DESKTOP="${HOME}/.local/share/applications"
INSTALL_ICONS="${HOME}/.local/share/icons/hicolor"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ASSETS="${SCRIPT_DIR}/assets"

echo "==> Compilando NSR-SSH (release)..."
cargo build --release -p nsr-ssh

mkdir -p "$INSTALL_BIN" "$INSTALL_DESKTOP"

echo "==> Instalando binário em ${INSTALL_BIN}/${BINARY_NAME}..."
cp "${SCRIPT_DIR}/target/release/${BINARY_NAME}" "${INSTALL_BIN}/${BINARY_NAME}"
chmod +x "${INSTALL_BIN}/${BINARY_NAME}"

echo "==> Instalando ícones..."
for SIZE in 16 32 48 64 128 256; do
    DIR="${INSTALL_ICONS}/${SIZE}x${SIZE}/apps"
    mkdir -p "$DIR"
    cp "${ASSETS}/icons/${BINARY_NAME}-${SIZE}.png" "${DIR}/${BINARY_NAME}.png"
done

# SVG escalável
SVG_DIR="${INSTALL_ICONS}/scalable/apps"
mkdir -p "$SVG_DIR"
cp "${ASSETS}/icons/${BINARY_NAME}.svg" "${SVG_DIR}/${BINARY_NAME}.svg"

echo "==> Instalando .desktop..."
# Atualiza o caminho do Exec para o binário instalado
sed "s|Exec=nsr-ssh|Exec=${INSTALL_BIN}/${BINARY_NAME}|g" \
    "${ASSETS}/${BINARY_NAME}.desktop" > "${INSTALL_DESKTOP}/${BINARY_NAME}.desktop"
chmod +x "${INSTALL_DESKTOP}/${BINARY_NAME}.desktop"

echo "==> Atualizando cache de ícones..."
gtk-update-icon-cache -f -t "${INSTALL_ICONS}" 2>/dev/null || true
update-desktop-database "$INSTALL_DESKTOP" 2>/dev/null || true

echo ""
echo "✓ NSR-SSH instalado com sucesso!"
echo "  Binário : ${INSTALL_BIN}/${BINARY_NAME}"
echo "  Menu    : ${INSTALL_DESKTOP}/${BINARY_NAME}.desktop"
echo ""
echo "  Se ${INSTALL_BIN} não estiver no PATH, adicione ao ~/.bashrc ou ~/.zshrc:"
echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
