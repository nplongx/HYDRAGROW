#pragma once
#include <map>
#include <string>

class Preferences {
    std::map<std::string, std::string> store_;
public:
    bool begin(const char* /*ns*/, bool /*readOnly*/ = false) { return true; }
    void end() {}
    bool putString(const char* key, const char* val) { store_[key] = val; return true; }
    String getString(const char* key, const char* defaultVal = "") {
        auto it = store_.find(key);
        return it != store_.end() ? String(it->second.c_str()) : String(defaultVal);
    }
    bool isKey(const char* key) { return store_.count(key) > 0; }
    void clear() { store_.clear(); }
};
