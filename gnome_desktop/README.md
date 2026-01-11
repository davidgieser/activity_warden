# GNOME Desktop:

This desktop application serves as the predominant interface with the `User Daemon`. The UI exposes methods to update the timers and allotted durations for each timer. Note also that the application can be locked down via a password interface.

## Setting Up The Application:

At `/usr/share/applications`, create a file with a `.desktop` file extension to create an application that can be opened from the application manager. A sample file is displayed beneath.

```
[Desktop Entry]
Type=Application
Name=Activity Warden
Comment=Monitor and Constrain Web Activity
Exec=/usr/local/bin/activity_warden %U
Terminal=false
Categories=Utility;
Icon=com.activity_warden.gui
StartupNotify=true
```

Note that the executable should be located in a root directory to make it more challenging for the applications to be compromised.

For an application icon, move images to the following paths: `/usr/share/icons/hicolor/128x128/apps/`.