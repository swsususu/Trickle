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

  # Declares the platform without asserting a version floor. Pinning one broke
  # installs on macOS 26 with "does not run on macOS versions other than
  # Ventura": older Homebrew releases mis-handle the comparison when their
  # version table predates the running system. The floor was a guess anyway, as
  # Trickle has only been run on macOS 26 and 27.
  depends_on macos: :any

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
