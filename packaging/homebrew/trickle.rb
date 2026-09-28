cask "trickle" do
  # Update both on every release. Get the checksum from the published asset:
  #   shasum -a 256 Trickle_<version>_universal.dmg
  # `:no_check` would avoid this step but silently accepts a tampered download.
  version "0.0.5"
  sha256 "698853b9669381cf414cce22ae219c33d2876a8ced2bf1b645e9e6696404d94b"

  url "https://github.com/swsususu/Trickle/releases/download/v#{version}/Trickle_#{version}_universal.dmg"
  name "Trickle"
  desc "Menu bar power monitor with battery health and system load"
  homepage "https://github.com/swsususu/Trickle"

  # Bare `depends_on :macos` declares the platform without a version floor,
  # which is what current Homebrew's style check requires. Do not use
  # `depends_on macos: :any`: Homebrew only accepts version symbols there, so
  # :any raises MacOSVersion::Error and invalidates the cask for everyone. A
  # version floor is also left out on purpose: Trickle has no tested one.
  depends_on :macos

  app "Trickle.app"

  zap trash: [
    "~/Library/Application Support/com.swsususu.trickle",
    "~/Library/Caches/com.swsususu.trickle",
    "~/Library/HTTPStorages/com.swsususu.trickle",
    "~/Library/Logs/com.swsususu.trickle",
    "~/Library/Preferences/com.swsususu.trickle.plist",
    "~/Library/Saved Application State/com.swsususu.trickle.savedState",
    "~/Library/WebKit/com.swsususu.trickle",
  ]

  # The app is ad-hoc signed, not notarised, so Gatekeeper blocks it on first
  # launch. Homebrew does NOT work around this: it sets the quarantine flag's
  # "user approved" bit, but macOS still refuses an app it cannot verify.
  # Verified by installing this cask and getting the "Apple could not verify"
  # dialog. Only notarisation with an Apple Developer certificate removes it.
  caveats <<~CAVEATS
    Trickle is not notarised yet, so macOS will refuse to open it the first
    time. Either right-click the app and choose Open, or run:

      xattr -dr com.apple.quarantine /Applications/Trickle.app
  CAVEATS
end
