// The open port used to communicate to the native messenger.
let port = browser.runtime.connectNative("com.activity_warden.firefox_native_messenger");

// On a click from the browser button, send the daemon a message.
browser.runtime.onMessage.addListener((message) => {
  if (message.action === "pingDaemon") {
    let sending = port.postMessage({ cmd: "ping" });
  }
});

port.onMessage.addListener((msg) => {
  console.log("Message from native host:", msg);
  // You can now relay this to other parts of the extension if needed

  switch (msg.type) {
    case "ACK":
      break;
    case "Close":
      let tab_id = parseInt(msg.tab_id);
      console.log(`Closing tab ${tab_id}`);
      browser.tabs.remove(tab_id).await;
      break
  }
});

port.onDisconnect.addListener(() => {
  console.error("Disconnected from native host");
  port = null;
});

// Listen to tab changes within FireFox.
let currentTab = null;

function updateActiveTab() {
  chrome.windows.getLastFocused({ populate: true }, (window) => {
    if (window.focused) {
      const activeTab = window.tabs.find(tab => tab.active);
      if (activeTab && (currentTab === null || activeTab.url !== currentTab.url)) {
        console.log("User switched to tab:", activeTab.url);
        currentTab = activeTab;

        let url = new URL(activeTab.url)

        // Send a message to the native listener.
        let event_obj = { 
          event_type: "focus_change",
          tab_id: activeTab.id, 
          tab_name: activeTab.title, 
          display_name: url.hostname,
        };

        console.log("Sending message: ", event_obj)
        port.postMessage(event_obj);
      }
    } else {
        let event_obj = { 
          event_type: "focus_lost",
        };
        currentTab = null
        
        console.log("Sending message: ", event_obj)
        port.postMessage(event_obj);
    }
  });
}

browser.tabs.onActivated.addListener(() => {
  updateActiveTab();
});

browser.tabs.onUpdated.addListener(
  (tabId, changeInfo, tab) => {
    console.log(tabId, changeInfo, tab)
    if (tab.active) {
      updateActiveTab();
    }
  },

  // TODO: Make the app continue to track if the tab is audible.
  {properties: ["url", "audible"]}
)

browser.windows.onFocusChanged.addListener(() => {
  updateActiveTab();
});