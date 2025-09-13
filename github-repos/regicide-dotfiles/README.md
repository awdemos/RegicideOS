# RegicideOS Dotfiles

User configuration files and dotfiles for RegicideOS. This repository is intended for users who want to customize their RegicideOS installation with additional applications and settings.

## Overview

This repository contains:
- Shell configurations (bash, zsh, fish)
- Terminal emulators and multiplexers
- Development environment setup
- Desktop environment customizations
- Application configurations
- System utilities and scripts

## Installation

### Quick Start
```bash
# Clone the repository
git clone https://github.com/regicideos/regicide-dotfiles.git
cd regicide-dotfiles

# Run the installer
./install.sh
```

### Manual Installation
```bash
# Create symlinks for specific configurations
stow bash
stow zsh
stow nvim
stow tmux
```

## Structure

```
├── bash/           # Bash shell configuration
├── zsh/            # Zsh shell configuration
├── fish/           # Fish shell configuration
├── nvim/           # Neovim configuration
├── tmux/           # Tmux multiplexer
├── git/            # Git configuration
├── ssh/            # SSH configuration
├── desktop/        # Desktop environment settings
├── devel/          # Development tools
├── utils/          # Utility scripts
└── install.sh      # Installation script
```

## Customization

### Shell Configuration
- Bash: Modern prompt, aliases, functions
- Zsh: Oh My Zsh integration, plugins
- Fish: Modern shell with autocompletion

### Development Environment
- Neovim: LSP, treesitter, plugins
- Git: User configuration, aliases
- SSH: Client configuration
- Rust: Toolchain configuration

### Desktop Environment
- Cosmic Desktop: Settings and themes
- Terminal: Profile configurations
- Shortcuts: Keybindings

## Usage

### Installation Script
```bash
# Interactive installation
./install.sh

# Non-interactive installation
./install.sh --non-interactive

# Install specific components
./install.sh --shell zsh --editor nvim --terminal alacritty
```

### Using GNU Stow
```bash
# Install all configurations
stow */

# Install specific components
stow bash nvim tmux

# Remove configurations
stow -D bash
```

## Features

### Shell Enhancements
- Modern prompts with git integration
- Intelligent command completion
- Useful aliases and functions
- Directory navigation shortcuts

### Development Tools
- Neovim with LSP and treesitter
- Git workflow improvements
- Project templates
- Development environment setup

### System Utilities
- Backup and restore scripts
- System maintenance tools
- Performance monitoring
- Security hardening

## Contributing

1. Fork the repository
2. Create your feature branch
3. Test your configurations
4. Update documentation
5. Submit a pull request

### Guidelines
- Keep configurations modular
- Use comments for complex settings
- Include installation instructions
- Test on fresh RegicideOS installation

## License

MIT License - See [LICENSE](LICENSE) for details.

## Support

- **Issues**: [GitHub Issues](https://github.com/regicideos/RegicideOS/issues)
- **Discussions**: [GitHub Discussions](https://github.com/regicideos/RegicideOS/discussions)
- **Documentation**: [RegicideOS Handbook](https://docs.regicideos.com)

---

**Personalize your RegicideOS experience** 🎨