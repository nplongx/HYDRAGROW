// docker/nodered/settings.js
// Tối thiểu: cho phép Node-RED khởi động trong container, MQTT broker connect
// trực tiếp từ flow (nodered-node-mqtt), không cần config thêm ở đây.
module.exports = {
  flowFile: 'flows.json',
  uiPort: 1880,
  editorTheme: {
    projects: { enabled: false },
  },
};
