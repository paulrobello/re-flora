# install cmake
sudo dnf install cmake

# install alsa-lib-devel for alsa.pc to work
# <https://www.reddit.com/r/linuxquestions/comments/pwxa8u/libasound2dev_on_fedora/>
sudo dnf install alsa-lib-devel

# install clang
sudo dnf install clang

# install clang-tools-extra
sudo dnf install clang-tools-extra

# install uv
curl -LsSf https://astral.sh/uv/install.sh | sh

# vulkan sdk
# <https://vulkan.lunarg.com/>

# Check if Vulkan SDK is already installed
if [ -z "$VULKAN_SDK" ]; then
    echo "Vulkan SDK not found. Installing..."

    # Define version and download URL
    VULKAN_VERSION="1.4.335.0"
    VULKAN_TARBALL="vulkansdk-linux-x86_64-${VULKAN_VERSION}.tar.xz"
    VULKAN_URL="https://sdk.lunarg.com/sdk/download/${VULKAN_VERSION}/linux/${VULKAN_TARBALL}"
    INSTALL_DIR="$HOME/vulkan-sdk"

    # Clean up any existing installation directory (may be corrupted)
    if [ -d "$INSTALL_DIR" ]; then
        echo "Removing existing Vulkan SDK directory (may be corrupted)..."
        rm -rf "$INSTALL_DIR"
    fi

    # Create fresh installation directory
    mkdir -p "$INSTALL_DIR"

    # Clean up any existing tarball (may be corrupted)
    if [ -f "$INSTALL_DIR/$VULKAN_TARBALL" ]; then
        echo "Removing existing tarball (may be corrupted)..."
        rm "$INSTALL_DIR/$VULKAN_TARBALL"
    fi

    # Download the tarball
    echo "Downloading Vulkan SDK ${VULKAN_VERSION}..."
    wget -P "$INSTALL_DIR" "$VULKAN_URL"

    # Extract the tarball (strip the outer directory to get a cleaner structure)
    echo "Extracting Vulkan SDK..."
    tar -xf "$INSTALL_DIR/$VULKAN_TARBALL" -C "$INSTALL_DIR" --strip-components=1

    # Remove the tarball
    echo "Cleaning up..."
    rm "$INSTALL_DIR/$VULKAN_TARBALL"

    # Add setup-env.sh to .zshrc if not already present
    SETUP_SCRIPT="$INSTALL_DIR/setup-env.sh"
    if [ -f "$SETUP_SCRIPT" ]; then
        if ! grep -q "vulkan-sdk.*setup-env.sh" ~/.zshrc 2>/dev/null; then
            echo "" >> ~/.zshrc
            echo "# Vulkan SDK" >> ~/.zshrc
            echo "source $SETUP_SCRIPT" >> ~/.zshrc
            echo "Added Vulkan SDK setup to ~/.zshrc"
        else
            echo "Vulkan SDK setup already present in ~/.zshrc"
        fi

        # Source it for current session
        source "$SETUP_SCRIPT"
        echo "Vulkan SDK installed successfully at $INSTALL_DIR"
    else
        echo "Warning: setup-env.sh not found at expected location"
    fi
else
    echo "Vulkan SDK already installed at: $VULKAN_SDK"
fi

# Reload shell configuration to apply all changes
echo "Reloading shell configuration..."
source ~/.zshrc
echo "Bootstrap complete!"
