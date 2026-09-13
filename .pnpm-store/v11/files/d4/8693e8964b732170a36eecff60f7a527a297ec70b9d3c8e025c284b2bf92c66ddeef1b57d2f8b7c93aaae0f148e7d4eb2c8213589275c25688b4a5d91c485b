'use strict';

Object.defineProperty(exports, '__esModule', { value: true });

var PropTypes = require('prop-types');
var qrcode = require('qrcode-generator');
var React = require('react');

function _extends() {
  return _extends = Object.assign ? Object.assign.bind() : function (n) {
    for (var e = 1; e < arguments.length; e++) {
      var t = arguments[e];
      for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]);
    }
    return n;
  }, _extends.apply(null, arguments);
}
function _objectWithoutProperties(e, t) {
  if (null == e) return {};
  var o,
    r,
    i = _objectWithoutPropertiesLoose(e, t);
  if (Object.getOwnPropertySymbols) {
    var n = Object.getOwnPropertySymbols(e);
    for (r = 0; r < n.length; r++) o = n[r], -1 === t.indexOf(o) && {}.propertyIsEnumerable.call(e, o) && (i[o] = e[o]);
  }
  return i;
}
function _objectWithoutPropertiesLoose(r, e) {
  if (null == r) return {};
  var t = {};
  for (var n in r) if ({}.hasOwnProperty.call(r, n)) {
    if (-1 !== e.indexOf(n)) continue;
    t[n] = r[n];
  }
  return t;
}

var _excluded$1 = ["bgColor", "bgD", "fgD", "fgColor", "size", "title", "viewBoxSize", "xmlns"];
var propTypes$1 = {
  bgColor: PropTypes.oneOfType([PropTypes.object, PropTypes.string]).isRequired,
  bgD: PropTypes.string.isRequired,
  fgColor: PropTypes.oneOfType([PropTypes.object, PropTypes.string]).isRequired,
  fgD: PropTypes.string.isRequired,
  size: PropTypes.number.isRequired,
  title: PropTypes.string,
  viewBoxSize: PropTypes.number.isRequired,
  xmlns: PropTypes.string
};
var QRCodeSvg = /*#__PURE__*/React.forwardRef(function (_ref, ref) {
  var bgColor = _ref.bgColor,
    bgD = _ref.bgD,
    fgD = _ref.fgD,
    fgColor = _ref.fgColor,
    size = _ref.size,
    title = _ref.title,
    viewBoxSize = _ref.viewBoxSize,
    _ref$xmlns = _ref.xmlns,
    xmlns = _ref$xmlns === void 0 ? "http://www.w3.org/2000/svg" : _ref$xmlns,
    props = _objectWithoutProperties(_ref, _excluded$1);
  return /*#__PURE__*/React.createElement("svg", _extends({}, props, {
    height: size,
    ref: ref,
    viewBox: "0 0 ".concat(viewBoxSize, " ").concat(viewBoxSize),
    width: size,
    xmlns: xmlns
  }), title ? /*#__PURE__*/React.createElement("title", null, title) : null, /*#__PURE__*/React.createElement("path", {
    d: bgD,
    fill: bgColor
  }), /*#__PURE__*/React.createElement("path", {
    d: fgD,
    fill: fgColor
  }));
});
QRCodeSvg.displayName = "QRCodeSvg";
QRCodeSvg.propTypes = propTypes$1;

var _excluded = ["bgColor", "fgColor", "level", "size", "value"];
qrcode.stringToBytes = function (s) {
  return Array.from(new TextEncoder().encode(s));
};
var propTypes = {
  bgColor: PropTypes.oneOfType([PropTypes.object, PropTypes.string]),
  fgColor: PropTypes.oneOfType([PropTypes.object, PropTypes.string]),
  level: PropTypes.string,
  size: PropTypes.number,
  value: PropTypes.string.isRequired
};
var QRCode = /*#__PURE__*/React.forwardRef(function (_ref, ref) {
  var _ref$bgColor = _ref.bgColor,
    bgColor = _ref$bgColor === void 0 ? "#FFFFFF" : _ref$bgColor,
    _ref$fgColor = _ref.fgColor,
    fgColor = _ref$fgColor === void 0 ? "#000000" : _ref$fgColor,
    _ref$level = _ref.level,
    level = _ref$level === void 0 ? "L" : _ref$level,
    _ref$size = _ref.size,
    size = _ref$size === void 0 ? 256 : _ref$size,
    value = _ref.value,
    props = _objectWithoutProperties(_ref, _excluded);
  var qr = qrcode(0, level);
  qr.addData(value);
  qr.make();
  var moduleCount = qr.getModuleCount();
  var cells = Array.from({
    length: moduleCount
  }, function (_, rowIndex) {
    return Array.from({
      length: moduleCount
    }, function (_, colIndex) {
      return qr.isDark(rowIndex, colIndex);
    });
  });
  return /*#__PURE__*/React.createElement(QRCodeSvg, _extends({}, props, {
    bgColor: bgColor,
    bgD: cells.map(function (row, rowIndex) {
      return row.map(function (cell, cellIndex) {
        return !cell ? "M ".concat(cellIndex, " ").concat(rowIndex, " l 1 0 0 1 -1 0 Z") : "";
      }).join(" ");
    }).join(" "),
    fgColor: fgColor,
    fgD: cells.map(function (row, rowIndex) {
      return row.map(function (cell, cellIndex) {
        return cell ? "M ".concat(cellIndex, " ").concat(rowIndex, " l 1 0 0 1 -1 0 Z") : "";
      }).join(" ");
    }).join(" "),
    ref: ref,
    size: size,
    viewBoxSize: moduleCount
  }));
});
QRCode.displayName = "QRCode";
QRCode.propTypes = propTypes;

exports.QRCode = QRCode;
exports.default = QRCode;
