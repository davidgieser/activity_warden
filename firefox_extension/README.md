# Firefox Extension

This component of the overall application tracks any tab changes within the Firefox web browser. All of these corresponding state changes are propagated to the User Daemon through the native messenging protocol. 

To update this code, some steps are required. One can test locally by temporarily installing the extension from a folder. For a more permanent solution, visit the [Firefox development hub](https://addons.mozilla.org/) and register the extension. Once it is approved, one can permanently download the extension. Simply download the XPI file and Firefox will register the extension. Note that the XPI file can be located at the [managing versions](https://addons.mozilla.org/en-US/developers/addon/9b4b2e41ccdb4805bcfd/versions) page.

To submit the new version to above link, increment the version number in the `manifest.json` file. Next, `cd` into the extension directory and run `zip -r ../activity_warden.zip .`. Submit the zip file to the site for an automated review.

To prevent the extension from being disabled, set the following `policies.json` file to lock down the extension further. Place this file at `/etc/firefox/policies/policies.json`.

```
{
  "policies": {
    "ExtensionSettings": {
      "*": {
        "installation_mode": "allowed"
      },
      "web_watcher@activity_warden.org": {
        "installation_mode": "force_installed",
        "install_url": "https://addons.mozilla.org/firefox/downloads/file/4631403/9b4b2e41ccdb4805bcfd-1.1.xpi"
      }
    }
  }
}
```


## Development:

Navigate to the [Firefox extension management](about:debugging#/runtime/this-firefox) page in Firefox. Next, you simply need to select the manifest to load the temporary extension.