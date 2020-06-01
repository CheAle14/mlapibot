Hi!  
I am responding because your image appears to contain an error assosciated with AnarchyGrabber, a malware that targets Discord.

### What is it?

AnarchyGrabber is a virus that attempts to gain access to your account and its information, such as your:

- Email,
- Plaintext password,
- Phone number,
- IP address

### What platforms are affected?

So far, only Windows and macOS.  

### How do I know if I'm infected?

AnarchyGrabber resides in Discord's startup code.  
On Windows: `%appdata%/discord/0.0.306/modules/discord_desktop_core`  
On macOS: `~/Library/Application Support/discord/0.0.306/modules/discord_desktop_core`  
Note that the version (0.0.306) is likely to change.

Within that folder, there should only be three files: `core.asar`, `index.js`, `package.json`.  
If there is an `4n4rchy` folder, then you've been infected.

### How do I remove it?

If you've been infected, you'll first want to make sure the 2FA on your account is yours, and redownload the backup codes to be sure you don't get locked out. Then change your password

1. Uninstall Discord using its proper uninstaller. 
2. Remove the `%appdata/discord%` or `~/Library/Application Support/discord` folder, depending on platform
3. Reinstall Discord

### What about the error?

If you don't appear to be infected, then its possible your antivirus is showing whats called a false-positive -- in other words, making a mistake.

If you've removed and reinstalled as per above, and thus have a clean installation that your antivirus is still indicating is infected, and you've verified you are not infected per above, then you can add an exemption to your antivirus to make it ignore the file/folder its getting a false-positive on.

This appears to be happening with Avast mainly, with it detecting even on the clean `index.js` file

- - -

I am a bot, if anything here is wrong or out of date, please message me.  