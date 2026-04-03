#!/usr/bin/env bash

set -e

detect_distro() {
  if [ -f /etc/os-release ]; then
    . /etc/os-release
    echo "$ID"
  elif [ -f /etc/fedora-release ]; then
    echo "fedora"
  elif [ -f /etc/debian_version ]; then
    echo "debian"
  else
    echo "unknown"
  fi
}

DISTRO=$(detect_distro)
echo "检测到发行版: $DISTRO"

case "$DISTRO" in
  ubuntu|debian|linuxmint|pop)
    echo "安装 Ubuntu/Debian 依赖..."
    sudo apt update
    # Test on Ubuntu 24.04
    sudo apt install -y \
      libudev-dev \
      gcc g++ clang libfontconfig-dev libwayland-dev \
      libwebkit2gtk-4.1-dev libxkbcommon-x11-dev libx11-xcb-dev \
      libssl-dev libzstd-dev \
      vulkan-validationlayers libvulkan1
    ;;
  fedora|rhel|rocky|almalinux|centos)
    echo "安装 Fedora/RHEL 依赖..."
    if command -v dnf &> /dev/null; then
      PKG_MANAGER="dnf"
    else
      PKG_MANAGER="yum"
    fi

    sudo $PKG_MANAGER install -y \
      systemd-devel \
      gcc gcc-c++ clang fontconfig-devel wayland-devel \
      webkit2gtk4.1-devel libxkbcommon-x11-devel libxcb-devel \
      openssl-devel libzstd-devel \
      vulkan-validation-layers vulkan-loader
    ;;
  arch|manjaro|endeavouros)
    echo "安装 Arch Linux 依赖..."
    sudo pacman -Sy --noconfirm \
      systemd \
      gcc clang fontconfig wayland \
      webkit2gtk-4.1 libxkbcommon-x11 libxcb \
      openssl zstd \
      vulkan-validation-layers vulkan-icd-loader
    ;;
  opensuse*)
    echo "安装 openSUSE 依赖..."
    sudo zypper install -y \
      systemd-devel \
      gcc gcc-c++ clang fontconfig-devel wayland-devel \
      libwebkit2gtk-4_1-0 libxkbcommon-x11-devel libxcb-devel \
      libopenssl-devel libzstd-devel \
      vulkan-validation-layers libvulkan1
    ;;
  *)
    echo "错误: 不支持的发行版 '$DISTRO'"
    echo "请手动安装以下依赖:"
    echo "  - gcc, g++, clang"
    echo "  - fontconfig 开发包"
    echo "  - wayland 开发包"
    echo "  - webkit2gtk-4.1 开发包"
    echo "  - libxkbcommon-x11 开发包"
    echo "  - libx11-xcb 或 libxcb 开发包"
    echo "  - openssl 开发包"
    echo "  - libzstd 开发包"
    echo "  - vulkan 相关包"
    exit 1
    ;;
esac

echo "依赖安装完成!"
