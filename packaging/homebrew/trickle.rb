cask "trickle" do
  # Update both on every release. Get the checksum from the published asset:
  #   shasum -a 256 Trickle_<version>_universal.dmg
  # `:no_check` would avoid this step but silently accepts a tampered download.
  version "0.0.1"
  sha256 "a456408d28d246aedd137b6ed6093c9723e22673281656c1bc1543cdf1bd4b43"

  url "https://github.com/swsususu/Trickle/releases/download/v#{version}/Trickle_#{version}_universal.dmg"
  name "Trickle"
  desc "Menu bar power monitor for battery health, power flow and charging"
  homepage "https://github.com/swsususu/Trickle"

  # Only macOS is relevant: the app reads IOKit power data and talks to the SMC.
  # This form already means "Ventura or newer". The floor is a conservative
  # guess, not a tested one — Trickle is verified on macOS 27 only. Adjust it
  # once older versions have actually been run.
  depends_on macos: :ventura

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
