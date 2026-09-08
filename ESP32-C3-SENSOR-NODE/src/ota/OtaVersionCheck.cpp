#include "OtaVersionCheck.h"

namespace OtaVersionCheck {

bool isUpdateAvailable(const String& currentVersion, const String& tagName) {
    if (tagName.length() == 0) return false;
    return tagName != currentVersion;
}

String findAssetDownloadUrl(JsonDocument& releaseDoc, const char* assetName) {
    JsonArray assets = releaseDoc["assets"].as<JsonArray>();
    if (assets.isNull()) return String("");

    for (JsonObject asset : assets) {
        const char* name = asset["name"] | "";
        if (strcmp(name, assetName) == 0) {
            const char* url = asset["browser_download_url"] | "";
            return String(url);
        }
    }
    return String("");
}

} // namespace OtaVersionCheck