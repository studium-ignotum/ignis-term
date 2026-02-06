cask "terminal-remote" do
  version "2.0.0"
  sha256 arm:   "PLACEHOLDER_ARM64_SHA256",
         intel: "PLACEHOLDER_X86_64_SHA256"

  on_arm do
    url "https://github.com/studium-ignotum/ignis-term/releases/download/v#{version}/terminal-remote-v#{version}-darwin-arm64.tar.gz"
  end
  on_intel do
    url "https://github.com/studium-ignotum/ignis-term/releases/download/v#{version}/terminal-remote-v#{version}-darwin-x86_64.tar.gz"
  end

  name "Terminal Remote"
  desc "Control any terminal session from anywhere via browser"
  homepage "https://github.com/studium-ignotum/ignis-term"

  # Install the .app bundle
  app "Terminal Remote.app"

  # Install binaries
  binary "relay-server", target: "#{HOMEBREW_PREFIX}/bin/terminal-remote-relay"
  binary "pty-proxy", target: "#{HOMEBREW_PREFIX}/bin/terminal-remote-pty-proxy"

  # Post-install: copy shell integration scripts and pty-proxy to install dir
  postflight do
    install_dir = "#{Dir.home}/.terminal-remote"
    bin_dir = "#{install_dir}/bin"
    FileUtils.mkdir_p(install_dir)
    FileUtils.mkdir_p(bin_dir)

    # Copy init scripts from the extracted archive
    ["init.zsh", "init.bash", "init.fish"].each do |script|
      source = "#{staged_path}/shell-integration/#{script}"
      FileUtils.cp(source, install_dir) if File.exist?(source)
    end

    # Symlink pty-proxy into install dir so shell integration can find it
    pty_proxy_target = "#{bin_dir}/pty-proxy"
    FileUtils.rm_f(pty_proxy_target)
    FileUtils.ln_sf("#{HOMEBREW_PREFIX}/bin/terminal-remote-pty-proxy", pty_proxy_target)
  end

  # Uninstall: clean up shell integration and config
  uninstall_postflight do
    install_dir = "#{Dir.home}/.terminal-remote"
    FileUtils.rm_rf(install_dir) if Dir.exist?(install_dir)

    # Clean up Unix socket
    socket = "/tmp/terminal-remote.sock"
    FileUtils.rm_f(socket) if File.exist?(socket)
  end

  depends_on formula: "cloudflared"
  depends_on macos: ">= :ventura"

  caveats <<~EOS
    Add shell integration to your shell configuration.
    Add this line to the END of your rc file (after oh-my-zsh, starship, etc.):

      Zsh (~/.zshrc):
        source ~/.terminal-remote/init.zsh

      Bash (~/.bashrc):
        source ~/.terminal-remote/init.bash

      Fish (~/.config/fish/config.fish):
        source ~/.terminal-remote/init.fish

    The pty-proxy is fully transparent — scroll, copy, mouse, and all
    terminal features work natively with your terminal emulator.
  EOS
end
