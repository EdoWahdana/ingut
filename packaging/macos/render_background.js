ObjC.import("AppKit");

function color(white) {
  return $.NSColor.colorWithCalibratedWhiteAlpha(white, 1);
}

function fill(x, y, w, h, nsColor) {
  nsColor.setFill;
  $.NSBezierPath.fillRect({ origin: { x: x, y: y }, size: { width: w, height: h } });
}

const width = 680;
const height = 420;
const logoPath = $.NSProcessInfo.processInfo.environment.objectForKey("LOGO").js;
const outputPath = $.NSProcessInfo.processInfo.environment.objectForKey("OUT").js;

const canvas = $.NSImage.alloc.initWithSize({ width: width, height: height });
canvas.lockFocus;

fill(0, 0, 250, height, $.NSColor.colorWithCalibratedRedGreenBlueAlpha(0.086, 0.086, 0.086, 1));
fill(250, 0, 430, height, $.NSColor.colorWithCalibratedRedGreenBlueAlpha(0.957, 0.957, 0.969, 1));

const logo = $.NSImage.alloc.initWithContentsOfFile(logoPath);
const side = 132;
logo.drawInRectFromRectOperationFraction(
  { origin: { x: (250 - side) / 2, y: (height - side) / 2 + 8 }, size: { width: side, height: side } },
  { origin: { x: 0, y: 0 }, size: { width: 0, height: 0 } },
  $.NSCompositingOperationSourceOver,
  1
);

function drawCentered(text, y, size, weight, white) {
  const font = $.NSFont.systemFontOfSizeWeight(size, weight);
  const attrs = $.NSMutableDictionary.alloc.init;
  attrs.setObjectForKey(font, $.NSFontAttributeName);
  attrs.setObjectForKey(color(white), $.NSForegroundColorAttributeName);
  const label = $(text);
  const measured = label.sizeWithAttributes(attrs);
  label.drawAtPointWithAttributes(
    { x: 250 + (430 - measured.width) / 2, y: y },
    attrs
  );
}

drawCentered("Ingut", height - 86, 28, $.NSFontWeightSemibold, 0.11);
drawCentered("DRAG ONTO APPLICATIONS TO INSTALL", 32, 11, $.NSFontWeightMedium, 0.45);

canvas.unlockFocus;

const tiff = canvas.TIFFRepresentation;
const rep = $.NSBitmapImageRep.imageRepWithData(tiff);
const png = rep.representationUsingTypeProperties($.NSBitmapImageFileTypePNG, $.NSDictionary.dictionary);
png.writeToFileAtomically(outputPath, true);
