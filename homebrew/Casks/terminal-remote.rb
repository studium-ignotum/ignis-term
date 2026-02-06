cask "terminal-remote" do
  version "2.0.0"
  sha256 arm:   "PLACEHOLDER_ARM64_SHA256",
         intel: "PLACEHOLDER_X86_64_SHA256"

  on_arm do
    url "https://github.com/studium-ignotum/iterm2-remote/releases/download/v#{version}/terminal-remote-v#{version}-darwin-arm64.tar.gz"
  end
  on_intel do
    url "https://github.com/studium-ignotum/iterm2-remote/releases/download/v#{version}/terminal-remote-v#{version}-darwin-x86_64.tar.gz"
  end

  name "Terminal Remote"
  desc "Control any terminal session from anywhere via browser"
  homepage "https://github.com/studium-ignotum/iterm2-remote"

  # Install the .app bundle
  app "Terminal Remote.app"

  # Install relay-server binary
  binary "relay-server", target: "#{HOMEBREW_PREFIX}/bin/terminal-remote-relay"

  # Post-install: copy shell integration scripts and create launcher
  postflight do
    install_dir = "#{Dir.home}/.terminal-remote"
    FileUtils.mkdir_p(install_dir)

    # Copy init scripts from the extracted archive
    ["init.zsh", "init.bash", "init.fish"].each do |script|
      source = "#{staged_path}/shell-integration/#{script}"
      FileUtils.cp(source, install_dir) if File.exist?(source)
    end

    # Create launcher script
    bin_dir = "#{install_dir}/bin"
    FileUtils.mkdir_p(bin_dir)

    launcher = "#{bin_dir}/terminal-remote-start"
    File.write(launcher, <<~SCRIPT)
      #!/bin/bash
      if ! pgrep -f "terminal-remote-relay" > /dev/null 2>&1; then
        terminal-remote-relay &
        sleep 0.5
      fi
      open "#{Dir.home}/Applications/Terminal Remote.app"
    SCRIPT
    FileUtils.chmod(0o755, launcher)
  end

  # Uninstall: clean up shell integration and config
  uninstall_postflight do
    install_dir = "#{Dir.home}/.terminal-remote"
    FileUtils.rm_rf(install_dir) if Dir.exist?(install_dir)
  end

  depends_on formula: "cloudflared"
  depends_on formula: "tmux"
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

    Start Terminal Remote:
      ~/.terminal-remote/bin/terminal-remote-start
  EOS
end
