#!/bin/sh

base_url="https://github.com/dmtrKovalenko/blendr/releases/download/latest"

# Function to check if a command exists
command_exists() {
  command -v "$1" >/dev/null 2>&1
}

# Function to print manual download instructions for unsupported platforms
print_manual_instructions() {
  echo "Error: Unsupported platform/architecture combination."
  echo "Please manually download the appropriate binary from the release page:"
  echo "https://github.com/dmtrKovalenko/blendr/releases/latest"
  exit 1
}

# Function to download and install the binary
install_binary() {
  local binary_url=$1
  local binary_file=$(basename "$binary_url")
  local install_dir=""

  if [ "$platform" = "darwin" ]; then
    install_dir="/usr/local/bin"
  elif [ "$platform" = "linux" ]; then
    install_dir="/usr/local/bin"
  elif [ "$platform" = "mingw"* ]; then
    install_dir="/usr/bin"
  fi

  echo "Downloading binary from '$binary_url'..."
  echo "curl -OL -H 'Authorization: token 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' "$binary_url""

  if command_exists "curl"; then
    curl -LO -H 'Authorization: token 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' "$binary_url"
  elif command_exists "wget"; then
    wget "$binary_url"
  else
    echo "Error: Neither 'curl' nor 'wget' found. Please install either of these tools."
    exit 1
  fi

  if [ ! -f "$binary_file" ]; then
    echo "Error: Download failed!"
    exit 1
  else
    echo "Downloaded binary '$binary_file' successfully!"
  fi

  tar -xzf "$binary_file"

  chmod +x blendr
  mv blendr "$install_dir"
  echo "Binary installed successfully in '$install_dir'!"
}

# Determine the platform and architecture
platform=$(uname | tr '[:upper:]' '[:lower:]')
arch=$(uname -m)

# Set the appropriate binary URL based on the platform and architecture
case "$platform" in
"darwin")
  case "$arch" in
  "arm64")
    binary_url="$base_url/latest/download/blendr-aarch64-apple-darwin.tar.gz"
    ;;
  "x86_64")
    binary_url="$base_url/latest/download/blendr-x86_64-apple-darwin.tar.gz"
    ;;
  *)
    print_manual_instructions "$base_url"
    ;;
  esac
  ;;
"linux")
  case "$arch" in
  "armv7l")
    binary_url="$base_url/blendr-arm-unknown-linux-gnueabihf.tar.gz"
    ;;
  "aarch64")
    binary_url="$base_url/blendr-aarch64-unknown-linux-gnu.tar.gz"
    ;;
  "i686")
    binary_url="$base_url/blendr-i686-unknown-linux-gnu.tar.gz"
    ;;
  "x86_64")
    binary_url="$base_url/blendr-x86_64-unknown-linux-gnu.tar.gz"
    ;;
  *)
    print_manual_instructions "$base_url"
    ;;
  esac
  ;;
"mingw"*)
  binary_url="$base_url/blendr-x86_64-pc-windows-msvc.tar.gz"
  ;;
*)
  print_manual_instructions "$base_url"
  ;;
esac

# Download and install the binary
install_binary "$binary_url"
