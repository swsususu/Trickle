cask "trickle" do
  # Update both on every release. Get the checksum from the published asset:
  #   shasum -a 256 Trickle_<version>_universal.dmg
  # `:no_check` would avoid this step but silently accepts a tampered download.
  version "0.0.3"
  sha256 "4fb6243caca1cbb31f91a53152aa3029649a840256347dbc91edb428a43c01eb"

  url "https://github.com/swsususu/Trickle/releases/download/v#{version}/Trickle_#{version}_universal.dmg"
  name "Trickle"
  desc "Menu bar power monitor for battery health, power flow and charging"
  homepage "https://github.com/swsususu/Trickle"

  # No `depends_on macos`: a cask only ever installs on macOS, so the stanza is
  # only for asserting a version floor, and Trickle has no tested one. Do not
  # reintroduce `macos: :any`: Homebrew only accepts version symbols such as
  # :tahoe or :sequoia, so :any raises MacOSVersion::Error and invalidates the
  # whole cask ("definition is invalid") for everyone installing it.

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
