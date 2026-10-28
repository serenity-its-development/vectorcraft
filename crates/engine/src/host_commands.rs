//! The assistant's command surface for the OneOS host bridge (`craft:command {method: "command.batch"}`).
//!
//! The host relays an allow-listed run: `{label, steps: [{name, args?}]}`. The whole run is judged BEFORE anything runs (an id
//! outside the list, an argument that names a file or an address, or a step reference to a step that has not run refuses the
//! run), every step runs through the same registry the menus, the control channel and the MCP server use (so a step the
//! drawing's state forbids is refused exactly as it would be for the person), and the run is ONE undo step named by `label`: a
//! step that fails takes back every step before it, so a run is all or nothing. There is no file-path or dialog command in the
//! list: files move only through the host.
//!
//! A later step may name the object an earlier step made: `{"$step": n}` stands for the `id` of step n's result,
//! `{"$step": n, "field": "..."}` for another field of it.
//!
//! This list is the one declared in the host's `platform-craft` registry as `vector` commands (pinned equal there).

use serde_json::{Value, json};

use crate::Session;

/// Registry commands the assistant may run. Each acts at once on the drawing (no dialog, no file picker, no pointer gesture),
/// and none is Undo or Redo: undoing a run is the person's.
pub const ASSISTANT_COMMANDS: &[&str] = &[
    // document.*
    "document.inspect",
    "document.node",
    "document.find",
    "document.json",
    "document.setUnits",
    "document.rulerOrigin",
    "document.setRulerOrigin",
    "document.resetRulerOrigin",
    // view.*
    "view.globalRulers",
    // edit.*
    "edit.dropSelection",
    "edit.clear",
    "edit.duplicate",
    // recolor.*
    "recolor.colors",
    "recolor.apply",
    "recolor.reduce",
    "recolor.randomize",
    // shape.*
    "shape.rectangle",
    "shape.ellipse",
    "shape.polygon",
    "shape.star",
    "shape.flare",
    "shape.line",
    "shape.spiral",
    "shape.arc",
    "shape.rectangularGrid",
    "shape.polarGrid",
    // path.*
    "path.create",
    // view.*
    "view.drawMode",
    // text.*
    "text.create",
    // dimension.*
    "dimension.create",
    "dimension.update",
    "dimension.applyToAll",
    // object.*
    "object.transform",
    "object.move",
    "object.rotate",
    "object.scale",
    "object.reflect",
    "object.shear",
    "object.transformAgain",
    "object.nudge",
    "object.arrange.bringToFront",
    "object.arrange.bringForward",
    "object.arrange.sendBackward",
    "object.arrange.sendToBack",
    "object.arrange.sendToCurrentLayer",
    "object.group",
    "object.ungroup",
    "object.lock",
    "object.unlockAll",
    "object.hide",
    "object.showAll",
    "object.compoundPath.make",
    "object.compoundPath.release",
    "object.clippingMask.make",
    "object.clippingMask.release",
    "object.clippingMask.editContents",
    "object.clippingMask.editMask",
    "object.isolate",
    "object.exitIsolation",
    "object.setProps",
    "object.align",
    "object.distribute",
    "object.distributeSpacing",
    "object.setBounds",
    "object.expandShape",
    "object.setLiveShape",
    // path.*
    "path.appendAnchor",
    "path.close",
    "path.moveAnchors",
    "path.setHandle",
    "path.moveAnchor",
    "path.setAnchors",
    "path.reverse",
    "path.join",
    "path.average",
    "path.convertAnchors",
    "path.deleteAnchors",
    "path.reshape",
    "path.insertAnchor",
    "path.cutAtAnchors",
    // select.*
    "select.all",
    "select.allOnArtboard",
    "select.none",
    "select.reselect",
    "select.inverse",
    "select.nextAbove",
    "select.nextBelow",
    "select.set",
    "select.add",
    "select.toggle",
    "select.key",
    "select.anchors",
    "select.segments",
    "select.anchorsMany",
    "select.color",
    "select.same.fillColor",
    "select.same.strokeColor",
    "select.same.strokeWeight",
    "select.same.fillAndStroke",
    "select.same.opacity",
    "select.same.blendingMode",
    "select.same.appearance",
    "select.same.shapeType",
    "select.object.allOnSameLayers",
    "select.object.clippingMasks",
    "select.object.textObjects",
    "select.object.strayPoints",
    "select.object.openPaths",
    "select.same.graphicStyle",
    "select.same.appearanceAttribute",
    "select.globalEdit",
    "select.globalEditOptions",
    "select.magicWand",
    // magicWand.*
    "magicWand.set",
    "magicWand.options",
    // paint.*
    "paint.setFill",
    "paint.setStroke",
    "paint.swap",
    "paint.default",
    "paint.toggleActive",
    "paint.none",
    // transparency.*
    "transparency.set",
    // stroke.*
    "stroke.set",
    "stroke.setAdvanced",
    "stroke.widthProfile.add",
    "stroke.widthProfile.delete",
    "stroke.widthProfile.reset",
    "stroke.widthProfile.list",
    // appearance.*
    "appearance.addFill",
    "appearance.addStroke",
    "appearance.clear",
    "appearance.reduceToBasic",
    "appearance.setItem",
    "appearance.removeItem",
    "appearance.addEffect",
    "appearance.duplicateItem",
    "appearance.moveItem",
    "appearance.copyFrom",
    "appearance.setActiveItem",
    "appearance.showAllHidden",
    "appearance.targetContents",
    "appearance.transfer",
    // graphicStyle.*
    "graphicStyle.apply",
    "graphicStyle.new",
    "graphicStyle.delete",
    "graphicStyle.duplicate",
    "graphicStyle.list",
    "graphicStyle.redefine",
    "graphicStyle.breakLink",
    "graphicStyle.rename",
    "graphicStyle.unused",
    "graphicStyle.sortByName",
    "graphicStyle.merge",
    "graphicStyle.move",
    "graphicStyle.setOptions",
    // swatch.*
    "swatch.new",
    "swatch.delete",
    "swatch.newGroup",
    "swatch.duplicate",
    "swatch.sortByName",
    "swatch.edit",
    "swatch.list",
    "swatch.move",
    "swatch.addUsedColors",
    "swatch.unused",
    "swatch.merge",
    "swatch.ungroup",
    "swatch.sortByKind",
    "swatch.spotOptions",
    "swatch.editGroup",
    // paint.*
    "paint.editGradient",
    "paint.setGradientGeom",
    // gradient.*
    "gradient.selectStop",
    // transparency.*
    "transparency.makeOpacityMask",
    "transparency.releaseOpacityMask",
    "transparency.disableOpacityMask",
    "transparency.enableOpacityMask",
    "transparency.unlinkOpacityMask",
    "transparency.linkOpacityMask",
    "transparency.setOpacityMask",
    "transparency.toggleNewMasksClipping",
    "transparency.toggleNewMasksInverted",
    "transparency.opacityMaskInfo",
    "transparency.info",
    "transparency.togglePageIsolatedBlending",
    "transparency.togglePageKnockoutGroup",
    // layer.*
    "layer.new",
    "layer.newSublayer",
    "layer.delete",
    "layer.duplicate",
    "layer.setCurrent",
    "layer.highlight",
    "layer.setProps",
    "layer.collectInNew",
    "layer.selectAll",
    "layer.move",
    // node.*
    "node.move",
    // artboard.*
    "artboard.new",
    "artboard.delete",
    "artboard.setProps",
    "artboard.fitToArt",
    "artboard.fitToSelection",
    // layer.*
    "layer.clippingMask.toggle",
    "layer.target",
    "layer.pasteRemembersLayers",
    "layer.merge",
    "layer.flatten",
    "layer.releaseToLayers",
    "layer.releaseToLayersBuild",
    "layer.reverse",
    "layer.template",
    "layer.hideOthers",
    "layer.showAll",
    "layer.outlineOthers",
    "layer.previewAll",
    "layer.lockOthers",
    "layer.unlockAll",
    "layer.locate",
    // path.*
    "path.freehand",
    "path.curvature",
    "path.curvatureEdit",
    "path.removeAnchor",
    "path.removeAnchors",
    "path.convertAnchor",
    "path.reshapeSegment",
    "path.split",
    "path.knife",
    "path.eraseRegion",
    "path.eraseSegments",
    "path.smoothRegion",
    "path.joinScrub",
    "path.blob",
    // object.*
    "object.distort",
    // paint.*
    "paint.sampleColor",
    // eyedropper.*
    "eyedropper.setOptions",
    // artboard.*
    "artboard.move",
    // effect.*
    "effect.apply",
    "effect.list",
    "effect.remove",
    "effect.setParams",
    "effect.lightingPreset",
    "effect.expandAppearance",
    "effect.duplicate",
    "effect.move",
    // object.*
    "object.pathfinder.unite",
    "object.pathfinder.minusFront",
    "object.pathfinder.intersect",
    "object.pathfinder.exclude",
    "object.pathfinder.divide",
    "object.pathfinder.trim",
    "object.pathfinder.merge",
    "object.pathfinder.crop",
    "object.pathfinder.outline",
    "object.pathfinder.minusBack",
    "object.pathfinder.trap",
    "object.path.outlineStroke",
    "object.path.offsetPath",
    "object.path.simplify",
    "object.path.addAnchorPoints",
    "object.path.divideObjectsBelow",
    "object.path.splitIntoGrid",
    "object.path.cleanUp",
    // type.*
    "type.orientation.vertical",
    "type.orientation.horizontal",
    "type.createOutlines",
    "type.scaleTextAttributes",
    "type.opticalMarginAlignment",
    "type.systemLayout",
    "type.fractionalWidths",
    // text.*
    "text.setText",
    "text.areaOptions",
    "text.reshapeArea",
    "text.fitHeadline",
    "text.setStyle",
    // type.*
    "type.ruby",
    // object.*
    "object.lock.above",
    "object.lock.otherLayers",
    "object.hide.above",
    "object.hide.otherLayers",
    "object.transformEach",
    "object.resetBoundingBox",
    "object.rasterize",
    "object.createObjectMosaic",
    "object.cropImage",
    "object.maskImage",
    "object.createTrimMarks",
    "object.shape.convertToShape",
    // artboard.*
    "artboard.convertToArtboards",
    "artboard.rearrange",
    // object.*
    "object.blend.make",
    "object.blend.release",
    "object.blend.options",
    "object.blend.expand",
    "object.blend.replaceSpine",
    "object.blend.reverseSpine",
    "object.blend.info",
    "object.blend.spine.moveAnchor",
    "object.blend.spine.convertAnchor",
    "object.blend.spine.reshapeSegment",
    "object.blend.spine.addAnchor",
    "object.blend.spine.removeAnchor",
    "object.blend.reverseFrontToBack",
    "object.envelope.makeWithWarp",
    "object.envelope.makeWithMesh",
    "object.envelope.makeWithTopObject",
    "object.envelope.resetWithWarp",
    "object.envelope.resetWithMesh",
    "object.envelope.info",
    "object.envelope.release",
    "object.envelope.options",
    "object.envelope.expand",
    "object.envelope.editContents",
    "object.envelope.setMeshPoint",
    "object.mesh.create",
    "object.mesh.setPointColor",
    "object.mesh.movePoint",
    "object.mesh.addLine",
    "object.mesh.deletePoint",
    "object.mesh.expand",
    // edit.*
    "edit.colors.invert",
    "edit.colors.toCMYK",
    "edit.colors.toGrayscale",
    "edit.colors.toRGB",
    "edit.colors.saturate",
    "edit.colors.adjustBalance",
    "edit.colors.blendFrontToBack",
    "edit.colors.blendHorizontally",
    "edit.colors.blendVertically",
    // color.*
    "color.harmony",
    // edit.*
    "edit.assignProfile",
    // object.*
    "object.convertDocumentColorMode",
    // color.*
    "color.convert",
    "color.gamutCheck",
    // view.*
    "view.proofSetup",
    "view.proofColors",
    "view.overprintPreview",
    "view.separationsPreview",
    // color.*
    "color.plates",
    // edit.*
    "edit.colors.overprintBlack",
    // swatch.*
    "swatch.setSpot",
    // type.*
    "type.changeCase",
    "type.smartPunctuation",
    "type.convertToAreaType",
    "type.convertToPointType",
    "type.pathOptions",
    "type.fillPlaceholder",
    "type.insert",
    // edit.*
    "edit.findReplace",
    "edit.findNext",
    // text.*
    "text.editRange",
    "text.setRangeStyle",
    "text.touchType",
    "text.getRange",
    "text.createInPath",
    // type.*
    "type.fitHeadline",
    "type.step",
    "type.size.increase",
    "type.size.decrease",
    "type.size.increaseByFive",
    "type.size.decreaseByFive",
    "type.resetTracking",
    "type.resetHorizontalScale",
    // text.*
    "text.discardEmpty",
    "text.insertInline",
    // charStyle.*
    "charStyle.list",
    "charStyle.new",
    "charStyle.apply",
    "charStyle.redefine",
    "charStyle.setAttrs",
    "charStyle.duplicate",
    "charStyle.rename",
    "charStyle.delete",
    // paraStyle.*
    "paraStyle.list",
    "paraStyle.new",
    "paraStyle.apply",
    "paraStyle.redefine",
    "paraStyle.setAttrs",
    "paraStyle.duplicate",
    "paraStyle.rename",
    "paraStyle.delete",
    // text.*
    "text.fonts",
    "text.replaceFont",
    // select.*
    "select.font",
    // text.*
    "text.fontList",
    "text.rescanFonts",
    "text.missingFonts",
    "text.thread.create",
    "text.thread.add",
    "text.thread.releaseSelection",
    "text.thread.remove",
    "text.thread.info",
    // object.*
    "object.textWrap.make",
    "object.textWrap.release",
    "object.textWrap.options",
    // graph.*
    "graph.create",
    "graph.setData",
    "graph.setType",
    "graph.design",
    "graph.column",
    "graph.marker",
    // document.*
    "document.rasterEffectsSettings",
    // view.*
    "view.saved.new",
    "view.saved.edit",
    "view.saved.list",
    "view.transparencyGrid",
    // transparency.*
    "transparency.editOpacityMask",
    "transparency.stopEditingOpacityMask",
    "transparency.viewOpacityMask",
    // text.*
    "text.tabs.set",
    "text.tabs.get",
    "text.tabs.clear",
    // select.*
    "select.same.symbolInstance",
    "select.same.fontFamily",
    "select.same.fontFamilyStyle",
    "select.same.fontFamilyStyleSize",
    "select.same.fontSize",
    "select.same.textFillColor",
    "select.same.textStrokeColor",
    "select.object.directionHandles",
    "select.object.pointText",
    "select.object.areaText",
    "select.object.brushStrokes",
    "select.object.bristleBrushStrokes",
    "select.save",
    "select.update",
    "select.recall",
    "select.editSaved",
    "select.savedList",
    // view.*
    "view.guides.make",
    "view.guides.release",
    "view.guides.lock",
    "view.guides.clear",
    // guide.*
    "guide.add",
    "guide.list",
    "guide.select",
    "guide.remove",
    "guide.move",
    "guide.setLayer",
    // variable.*
    "variable.define",
    "variable.delete",
    "variable.rename",
    "variable.list",
    "variable.makeTextDynamic",
    "variable.makeVisibilityDynamic",
    "variable.makeImageDynamic",
    "variable.makeGraphDynamic",
    "variable.bind",
    "variable.unbind",
    "variable.lock",
    "variable.highlight",
    // dataset.*
    "dataset.new",
    "dataset.set",
    "dataset.rename",
    "dataset.capture",
    "dataset.update",
    "dataset.delete",
    "dataset.list",
    "dataset.select",
    "dataset.next",
    "dataset.prev",
    // document.*
    "document.info",
    // object.*
    "object.makePixelPerfect",
    // artboard.*
    "artboard.reorder",
    "artboard.duplicate",
    "artboard.copy",
    "artboard.cut",
    // text.*
    "text.setFormat",
    // shapeBuilder.*
    "shapeBuilder.merge",
    "shapeBuilder.regions",
    // livePaint.*
    "livePaint.make",
    "livePaint.merge",
    "livePaint.release",
    "livePaint.expand",
    "livePaint.gapOptions",
    "livePaint.fill",
    "livePaint.strokeEdge",
    "livePaint.info",
    // imageTrace.*
    "imageTrace.make",
    "imageTrace.makeAndExpand",
    "imageTrace.release",
    "imageTrace.expand",
    "imageTrace.setView",
    "imageTrace.presets",
    // shaper.*
    "shaper.scribble",
    "shaper.select",
    "shaper.release",
    "shaper.expand",
    // brush.*
    "brush.list",
    "brush.get",
    "brush.selectionArt",
    "brush.apply",
    "brush.remove",
    "brush.setCurrent",
    "brush.new",
    "brush.delete",
    "brush.move",
    "brush.duplicate",
    "brush.unused",
    "brush.options",
    // object.*
    "object.expandBrush",
    // brush.*
    "brush.freehand",
    // symbol.*
    "symbol.list",
    "symbol.new",
    "symbol.options",
    "symbol.place",
    "symbol.breakLink",
    "symbol.edit",
    "symbol.update",
    "symbol.delete",
    "symbol.duplicate",
    "symbol.replace",
    "symbol.setCurrent",
    "symbol.selectInstances",
    "symbol.spray",
    "symbol.adjust",
    // object.*
    "object.pattern.make",
    "object.pattern.edit",
    "object.pattern.done",
    "object.pattern.cancel",
    "object.pattern.saveCopy",
    // pattern.*
    "pattern.options",
    "pattern.delete",
    "pattern.list",
    "pattern.transform",
    // object.*
    "object.repeat.radial",
    "object.repeat.grid",
    "object.repeat.mirror",
    "object.repeat.release",
    "object.repeat.options",
    "object.repeat.expand",
    "object.intertwine.make",
    "object.intertwine.edit",
    "object.intertwine.release",
    "object.intertwine.arrange",
    "object.objectsOnPath.attach",
    "object.objectsOnPath.options",
    "object.objectsOnPath.expand",
    // stroke.*
    "stroke.widthPoint.set",
    "stroke.widthPoint.remove",
    "stroke.widthPoint.copy",
    "stroke.widthProfile.set",
    // object.*
    "object.liquify",
    "object.puppetWarp",
    "object.puppetWarp.pins",
    // perspective.*
    "perspective.grid.set",
    "perspective.grid.preset",
    "perspective.grid.show",
    "perspective.plane.set",
    "perspective.attach",
    "perspective.release",
    "perspective.move",
    "perspective.draw",
    "perspective.transform",
    "perspective.nudge",
    "perspective.plane.move",
    "perspective.plane.matchObject",
    "perspective.editText",
    // paint.*
    "paint.invert",
    "paint.complement",
    "paint.lastColor",
    "paint.lastGradient",
    "paint.recent",
    "paint.proxies",
    // object.*
    "object.setOverprint",
    // attributes.*
    "attributes.info",
    // paint.*
    "paint.freeform.addPoint",
    "paint.freeform.setPoint",
    "paint.freeform.deletePoint",
    "paint.freeform.addLine",
    "paint.freeform.splitLine",
    "paint.freeform.selectPoint",
    "paint.freeform.get",
    // object.*
    "object.flattenTransparency",
    // flattener.*
    "flattener.presets.list",
    "flattener.presets.delete",
    "flattener.preview",
    // object.*
    "object.expand",
    "object.expand.info",
    // attributes.*
    "attributes.set",
    // path.*
    "path.setFillRule",
    // svgInteractivity.*
    "svgInteractivity.info",
    "svgInteractivity.setEvent",
    "svgInteractivity.removeEvent",
    "svgInteractivity.clearEvents",
    // appearance.*
    "appearance.setNewArtBasic",
    "appearance.newArt",
    // colorTheme.*
    "colorTheme.list",
    "colorTheme.delete",
    "colorTheme.addToSwatches",
    // document.*
    "document.setup",
    // image.*
    "image.info",
    // object.*
    "object.slice.make",
    "object.slice.release",
    "object.slice.fromGuides",
    "object.slice.fromSelection",
    "object.slice.duplicate",
    "object.slice.combine",
    "object.slice.divide",
    "object.slice.deleteAll",
    "object.slice.options",
    "object.slice.clipToArtboard",
    // view.*
    "view.slices.hide",
    "view.slices.lock",
    // slice.*
    "slice.list",
    // select.*
    "select.object.slices",
    // object.*
    "object.slice.create",
    "object.slice.setRect",
    "object.slice.move",
    "object.slice.delete",
    "object.slice.select",
    // css.*
    "css.selection",
    "css.generate",
    // view.*
    "view.printTiling",
    // path.*
    "path.mirrorCut",
    "path.lineCut",
    "path.rectCut",
    // object.*
    "object.vectorHalftone",
    // perspective.*
    "perspective.grid.get",
    "perspective.grid.define",
    "perspective.presets.list",
    "perspective.presets.delete",
    "perspective.widget.options",
    "perspective.grid.lock",
    "perspective.grid.lockStation",
    "perspective.grid.snap",
    "perspective.grid.rulers",
    // tool.*
    "tool.select",
    "tool.setOption",
    // view.*
    "view.goto1",
    "view.goto2",
    "view.goto3",
    "view.goto4",
    "view.goto5",
    "view.goto6",
    "view.goto7",
    "view.goto8",
    "view.goto9",
    "view.goto10",
    // select.*
    "select.recall1",
    "select.recall2",
    "select.recall3",
    "select.recall4",
    "select.recall5",
    "select.recall6",
    "select.recall7",
    "select.recall8",
    "select.recall9",
    "select.recall10",
    "select.recall11",
    "select.recall12",
    "select.recall13",
    "select.recall14",
    "select.recall15",
    "select.recall16",
    "select.recall17",
    "select.recall18",
    "select.recall19",
    "select.recall20",
    "select.recall21",
    "select.recall22",
    "select.recall23",
    "select.recall24",
    "select.recall25",
    // type.*
    "type.recentFont1",
    "type.recentFont2",
    "type.recentFont3",
    "type.recentFont4",
    "type.recentFont5",
    "type.recentFont6",
    "type.recentFont7",
    "type.recentFont8",
    "type.recentFont9",
    "type.recentFont10",
    "type.recentFont11",
    "type.recentFont12",
    "type.recentFont13",
    "type.recentFont14",
    "type.recentFont15",
    "type.findFont",
    // ui.*
    "ui.missingFontsDialog",
    "ui.findFontsInFolder",
    // edit.*
    "edit.preferences",
    "edit.keyboardShortcuts",
    // shortcuts.*
    "shortcuts.set",
    "shortcuts.list",
    "shortcuts.conflicts",
    "shortcuts.reset",
    "shortcuts.preset",
    // view.*
    "view.outline",
    "view.pixelPreview",
    "view.trimView",
    "view.cornerWidget",
    "view.snapToPixel",
    "view.snapToGlyph",
    "view.preciseCursors",
    "view.textThreads",
    "view.gradientAnnotator",
    "view.livePaintGaps",
    // type.*
    "type.hiddenCharacters",
    "type.alignLeft",
    "type.alignCenter",
    "type.alignRight",
    "type.justifyLeft",
    "type.justifyAll",
    "type.applyBullets",
    "type.applyNumbers",
    // align.*
    "align.horizontalLeft",
    "align.horizontalCenter",
    "align.horizontalRight",
    "align.horizontalVerticalCenter",
    "align.verticalTop",
    "align.verticalCenter",
    "align.verticalBottom",
    "align.distributeVerticalTop",
    "align.distributeVerticalCenter",
    "align.distributeVerticalBottom",
    "align.distributeHorizontalLeft",
    "align.distributeHorizontalCenter",
    "align.distributeHorizontalRight",
    "align.distributeVerticalSpace",
    "align.distributeHorizontalSpace",
    // type.*
    "type.bold",
    "type.italic",
    // effect.*
    "effect.last",
    // view.*
    "view.zoomIn",
    "view.zoomOut",
    "view.fitArtboard",
    "view.fitAll",
    "view.actualSize",
    "view.setZoom",
    "view.goToArtboard",
    "view.edges",
    "view.artboards",
    "view.template",
    "view.rulers",
    "view.videoRulers",
    "view.boundingBox",
    "view.guides",
    "view.smartGuides",
    "view.grid",
    "view.snapToGrid",
    "view.snapToPoint",
    "view.presentation",
    "view.screenMode",
    "view.rotate",
    "view.rotateToSelection",
    "view.rotateReset",
    // window.*
    "window.control",
    "window.toolbar",
    "window.toolbarColumns",
    "window.toolbarAdvanced",
    "window.floatTools",
    // toolbar.*
    "toolbar.show",
    // window.*
    "window.editToolbar",
    // toolbar.*
    "toolbar.addTool",
    "toolbar.removeTool",
    "toolbar.reset",
    // window.*
    "window.taskBar",
    "window.taskBar.pin",
    "window.taskBar.reset",
    "window.dock",
    "window.panels",
    "window.panel",
    "window.panel.layout",
    "window.panel.move",
    "window.panel.activate",
    "window.panel.close",
    "window.panel.collapse",
    "window.panel.float",
    "window.panel.dock",
    "window.collapseDock",
    "window.brightness",
    "window.workspace",
    "window.workspace.reset",
    "window.workspace.new",
    "window.workspace.manage",
    "window.workspace.delete",
    "window.workspace.rename",
    "window.workspace.list",
    "window.newWindow",
    // effect.*
    "effect.dialog",
    // ui.*
    "ui.paramDialog",
    "ui.recolorDialog",
    "ui.cropImage",
    // effect.*
    "effect.applyLast",
    // ui.*
    "ui.swatchOptions",
    "ui.newSwatch",
    "ui.newColorGroup",
    "ui.colorPicker",
    "ui.sampleScreenColor",
    "ui.newBrush",
    "ui.brushOptions",
    "ui.graphicStyleOptions",
    "ui.symbolOptions",
    // tool.*
    "tool.options",
    // ui.*
    "ui.colorGuideOptions",
    "ui.colorBalanceDialog",
    "ui.saturateDialog",
    // window.*
    "window.userSwatchLibrary1",
    "window.userSwatchLibrary2",
    "window.userSwatchLibrary3",
    "window.userSwatchLibrary4",
    "window.userSwatchLibrary5",
    "window.userSwatchLibrary6",
    "window.userSwatchLibrary7",
    "window.userSwatchLibrary8",
    "window.userSwatchLibrary9",
    "window.userSwatchLibrary10",
    // ui.*
    "ui.mergeGraphicStyles",
    "ui.layerOptions",
    "ui.newLayer",
    "ui.newArtboard",
    "ui.layersPanelOptions",
    "ui.layersExpand",
    "ui.layersFilter",
    "ui.tileEdgeColor",
    "ui.flattenTransparencyDialog",
    "ui.flattenerPresetsDialog",
    "ui.flattenerPreview",
    // window.*
    "window.userGraphicStyleLibrary1",
    "window.userGraphicStyleLibrary2",
    "window.userGraphicStyleLibrary3",
    "window.userGraphicStyleLibrary4",
    "window.userGraphicStyleLibrary5",
    "window.userGraphicStyleLibrary6",
    "window.userGraphicStyleLibrary7",
    "window.userGraphicStyleLibrary8",
    "window.userGraphicStyleLibrary9",
    "window.userGraphicStyleLibrary10",
    "window.userSymbolLibrary1",
    "window.userSymbolLibrary2",
    "window.userSymbolLibrary3",
    "window.userSymbolLibrary4",
    "window.userSymbolLibrary5",
    "window.userSymbolLibrary6",
    "window.userSymbolLibrary7",
    "window.userSymbolLibrary8",
    "window.userSymbolLibrary9",
    "window.userSymbolLibrary10",
    // ui.*
    "ui.expandDialog",
    // attributes.*
    "attributes.openUrl",
    // ui.*
    "ui.spotColors",
    "ui.menuDialog",
    "ui.widthPointEdit",
    "ui.corners",
    "ui.simplifyAuto",
    "ui.simplifyMoreOptions",
    "ui.justification",
    "ui.hyphenation",
    "ui.colorGuideLimit",
    "ui.colorPanel",
    "ui.rasterEffectsSettingsDialog",
    "ui.effectGalleryDialog",
    "ui.pdfPresetsDialog",
    "ui.pdfPresetDialog",
    "ui.dxfOptionsDialog",
    "ui.placementOptionsDialog",
    "ui.findLinksInFolder",
    "ui.batchDialog",
    "ui.epsOptionsDialog",
    // css.*
    "css.copy",
    // ui.*
    "ui.perspectiveGridDialog",
    "ui.perspectivePresetsDialog",
    "ui.perspectiveUserPreset1.1",
    "ui.perspectiveUserPreset1.2",
    "ui.perspectiveUserPreset1.3",
    "ui.perspectiveUserPreset1.4",
    "ui.perspectiveUserPreset1.5",
    "ui.perspectiveUserPreset2.1",
    "ui.perspectiveUserPreset2.2",
    "ui.perspectiveUserPreset2.3",
    "ui.perspectiveUserPreset2.4",
    "ui.perspectiveUserPreset2.5",
    "ui.perspectiveUserPreset3.1",
    "ui.perspectiveUserPreset3.2",
    "ui.perspectiveUserPreset3.3",
    "ui.perspectiveUserPreset3.4",
    "ui.perspectiveUserPreset3.5",
    "ui.blendOptions",
    "ui.livePaintGapOptions",
    "ui.perspectivePlane",
];

/// Commands the window itself provides (they take arguments, which the registry's commands do not, or answer in a shape of their own):
/// `inspect` reads the drawing (artboards, layers as a tree of objects with id, name, kind, bounds, fill and stroke, the selection and what Undo
/// would say) to a depth; `render` reads an artboard (index, default 0) as a PNG, base64, at most 1600 pixels on a side.
pub const ARGUMENT_COMMANDS: &[&str] = &["inspect", "render"];

/// Every command the app has and the assistant may NOT run, with the reason: together with `ASSISTANT_COMMANDS` this is the whole command set of the window
/// (a test holds the two to the registry, so a command can neither be forgotten nor listed twice).
pub const LEFT_OUT: &[(&str, &[&str])] = &[
    (
        "Files move only through the host: open, save, export, place, package, print and the like are the host's, never a command id the assistant runs.",
        &[
            "file.new", "file.close", "clipboard.exportSvg", "clipboard.importSvg", "clipboard.conflicts", "clipboard.exportPng",
            "clipboard.exportPdf", "clipboard.exportText", "clipboard.flavours", "clipboard.importImage", "clipboard.importText",
            "clipboard.importPdf", "clipboard.importEmf", "clipboard.importEps", "document.open", "file.list", "document.serialize",
            "document.export", "document.exportSelection", "document.exportForScreens", "document.exportPresets.list", "document.exportPresets.save",
            "document.exportPresets.delete", "document.formats", "document.pdfInfo", "document.exportForOffice", "document.exportSettings",
            "document.save", "file.saveAs", "file.saveCopy", "file.saveAsTemplate", "file.newFromTemplate", "file.revert", "file.formatOptions",
            "color.loadProfile", "color.removeProfile", "file.closeAll", "file.documentColorMode", "file.info", "dataset.import", "dataset.export",
            "variable.loadLibrary", "document.exportPdf", "document.pdfSettings", "file.newPresets", "file.newPresets.save",
            "file.newPresets.delete", "file.place", "file.place.info", "file.place.queue", "links.check", "links.update", "links.updateChanged",
            "links.relink", "links.list", "links.goTo", "links.embed", "links.unembed", "links.info", "links.placementOptions", "links.findFiles",
            "pdf.preset.list", "pdf.preset.save", "pdf.preset.delete", "pdf.preset.export", "pdf.preset.import", "document.exportDxf",
            "document.exportEps", "file.package", "print.setup", "print.preview", "file.print", "file.recovery.save", "file.recovery.list",
            "file.recovery.restore", "file.recovery.discard", "document.dxfInfo", "document.exportForWeb", "document.exportForWeb.preview",
            "webExport.settings", "webExport.presets.list", "webExport.presets.save", "webExport.presets.delete", "assets.add", "assets.remove",
            "assets.rename", "assets.list", "assets.export", "assets.settings.set", "css.export", "print.presets.list", "print.presets.save",
            "print.presets.delete", "print.presets.export", "print.presets.import", "print.tiling.set", "print.tiling", "plugin.list", "plugin.info",
            "plugin.run", "plugin.install", "plugin.inspect", "plugin.remove", "plugin.reload", "script.check", "importMatches.list",
            "importMatches.set", "importMatches.reset", "file.open", "file.save", "file.export", "file.exportForScreens", "file.openRecent1",
            "file.openRecent2", "file.openRecent3", "file.openRecent4", "file.openRecent5", "file.openRecent6", "file.openRecent7",
            "file.openRecent8", "file.openRecent9", "file.openRecent10", "file.clearRecent", "file.recentFiles", "file.export.svg",
            "file.export.png", "file.documentSetup", "file.newDialog", "file.newWithoutDialog", "file.export.pdf", "file.exportAs",
            "file.openRecent11", "file.openRecent12", "file.openRecent13", "file.openRecent14", "file.openRecent15", "file.openRecent16",
            "file.openRecent17", "file.openRecent18", "file.openRecent19", "file.openRecent20", "file.openRecent21", "file.openRecent22",
            "file.openRecent23", "file.openRecent24", "file.openRecent25", "file.openRecent26", "file.openRecent27", "file.openRecent28",
            "file.openRecent29", "file.openRecent30", "file.reveal", "file.cancelExport", "file.exportInBackground", "links.editOriginal",
            "links.reveal", "file.showPackage", "file.saveForWeb", "file.saveForWeb.browser", "file.exportSelection", "print.printers",
            "print.printerSetup", "plugin.dialog", "plugin.cancelRuns", "plugin.hideRuns",
        ],
    ),
    (
        "The window shows the host's one file: commands that switch to another document are refused.",
        &[
            "document.activate",
        ],
    ),
    (
        "Undo, redo and the history belong to the person: a run is one undo step, and undoing it is theirs.",
        &[
            "edit.undo", "edit.redo", "edit.clearHistory", "edit.undoGroup.begin", "edit.undoGroup.end",
        ],
    ),
    (
        "The system clipboard is the person's (copy, cut and paste read or write what they hold outside the app).",
        &[
            "edit.cut", "edit.copy", "edit.paste", "edit.pasteInFront", "edit.pasteInBack", "edit.pasteInPlace", "edit.pasteOnAllArtboards",
            "edit.pasteWithoutFormatting",
        ],
    ),
    (
        "Nested runs: a command that runs a list of commands or an action would hide steps from the allow-list (a run is already a list of steps).",
        &[
            "command.batch", "action.batch",
        ],
    ),
    (
        "Preferences, appearance, shortcut sets and libraries (stored in the person's browser) are theirs, not the drawing's.",
        &[
            "edit.colorSettings", "help.discord", "help.website", "help.appPage", "help.github", "help.links", "prefs.get", "prefs.set",
            "prefs.reset", "prefs.list", "swatch.library.list", "swatch.library.get", "swatch.library.add", "swatch.resetDefaults",
            "swatch.library.save", "swatch.library.load", "swatch.library.copyToUser", "graphicStyle.libraries", "graphicStyle.library",
            "graphicStyle.addFromLibrary", "graphicStyle.saveLibrary", "graphicStyle.loadLibrary", "symbol.libraries", "symbol.library",
            "symbol.addFromLibrary", "symbol.saveLibrary", "symbol.loadLibrary", "library.create", "library.rename", "library.delete", "library.add",
            "library.removeItem", "library.createGroup", "library.renameGroup", "library.deleteGroup", "library.moveItem", "library.moveGroup",
            "library.export", "library.import", "shortcutSet.read", "app.language", "app.home", "help.about", "help.commandPalette", "app.quit",
        ],
    ),
    (
        "Its name is a file operation (open, save, export, import, print, library, plug-in, script).",
        &[
            "text.findFontFiles", "text.addFontFiles", "flattener.presets.save", "flattener.presets.import", "flattener.presets.export",
            "library.list", "library.get", "library.setCurrent", "library.use", "svgInteractivity.addFile", "svgInteractivity.removeFile",
            "colorTheme.save", "perspective.presets.save", "perspective.presets.export", "perspective.presets.import", "shortcuts.export",
            "shortcuts.import", "ui.shortcutImportDialog", "window.swatchLibrary", "window.swatchLibrary.other", "ui.libraryPanel",
            "ui.libraryPanel.locate", "ui.saveSwatchLibrary", "window.graphicStyleLibrary", "window.graphicStyleLibrary.other",
            "ui.saveGraphicStyleLibrary", "window.symbolLibrary", "window.symbolLibrary.other", "ui.saveSymbolLibrary", "ui.javascriptFilesDialog",
            "ui.savePdfDialog", "ui.fileInfoDialog", "ui.packageDialog", "docInfo.save", "css.exportFile", "ui.printPresetsDialog",
            "ui.printPresetDialog", "ui.installPlugin", "ui.checkScript", "ui.savePerspectivePreset",
        ],
    ),
];

/// Every command id a run may name, in the order the host's registry declares them.
pub fn allowed() -> Vec<&'static str> {
    ASSISTANT_COMMANDS.iter().chain(ARGUMENT_COMMANDS).copied().collect()
}

/// The most steps one run may hold.
pub const MAX_STEPS: usize = 50;
/// The longest label of a run.
pub const MAX_LABEL: usize = 120;
/// The most levels `inspect` walks into.
const MAX_INSPECT_DEPTH: u64 = 8;
/// The longest side of a `render`.
const MAX_RENDER_SIDE: f64 = 1600.0;
/// Argument names that would point a command at a file or a network address (or carry a file's bytes): refused anywhere in a step's arguments.
const FORBIDDEN_ARGS: &[&str] = &["path", "dir", "folder", "file", "files", "url", "uri", "href", "server", "dataBase64", "base64"];

/// The first forbidden argument name anywhere in `v`.
fn forbidden_arg(v: &Value) -> Option<String> {
    match v {
        Value::Object(o) => o.iter().find_map(|(k, x)| if FORBIDDEN_ARGS.contains(&k.as_str()) { Some(k.clone()) } else { forbidden_arg(x) }),
        Value::Array(a) => a.iter().find_map(forbidden_arg),
        _ => None,
    }
}

/// A run, judged: its label and its steps (the command id and its arguments).
#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    pub label: String,
    pub steps: Vec<(String, Value)>,
}

/// Judge a `command.batch` request before anything runs.
pub fn judge(params: &Value) -> Result<Run, String> {
    let label = params.get("label").and_then(Value::as_str).map(str::trim).unwrap_or("");
    if label.chars().count() > MAX_LABEL {
        return Err(format!("label-too-long: a run's label is at most {MAX_LABEL} characters"));
    }
    let steps = params.get("steps").and_then(Value::as_array).ok_or("run-empty: a run states its steps")?;
    if steps.is_empty() {
        return Err("run-empty: a run states at least one command".into());
    }
    if steps.len() > MAX_STEPS {
        return Err(format!("run-too-long: a run is at most {MAX_STEPS} steps"));
    }
    let mut named = Vec::with_capacity(steps.len());
    for (i, step) in steps.iter().enumerate() {
        let name = step.get("name").and_then(Value::as_str).unwrap_or("");
        if !ASSISTANT_COMMANDS.contains(&name) && !ARGUMENT_COMMANDS.contains(&name) {
            return Err(format!("command-not-allowed: Vector does not allow the command '{name}' -- nothing was run"));
        }
        let args = step.get("args").cloned().unwrap_or_else(|| json!({}));
        if !(args.is_object() || args.is_null()) {
            return Err(format!("argument-not-allowed: step {} ('{name}') takes its arguments as an object -- nothing was run", i + 1));
        }
        if let Some(k) = forbidden_arg(&args) {
            return Err(format!("argument-not-allowed: step '{name}' names '{k}', and a run does not reach files or addresses -- nothing was run"));
        }
        named.push((name.to_string(), if args.is_null() { json!({}) } else { args }));
    }
    let label = if label.is_empty() { "Assistant" } else { label };
    Ok(Run { label: label.to_string(), steps: named })
}

/// Replace every `{"$step": n}` / `{"$step": n, "field": "..."}` in `v` with that field (default `id`) of `results[n]`, the
/// result of an earlier step.
pub fn resolve_step_refs(v: &Value, results: &[Value], step: usize, depth: usize) -> Result<Value, String> {
    if depth > 64 {
        return Err(format!("step {}: arguments nest deeper than 64 levels", step + 1));
    }
    match v {
        Value::Object(o) if o.contains_key("$step") => {
            if let Some(k) = o.keys().find(|k| *k != "$step" && *k != "field") {
                return Err(format!("step {}: a `$step` reference takes only `field`, not `{k}`", step + 1));
            }
            let n = o.get("$step").and_then(Value::as_u64).ok_or_else(|| format!("step {}: `$step` must be a step index (0, 1, ...)", step + 1))?;
            let Some(result) = usize::try_from(n).ok().filter(|n| *n < step).and_then(|n| results.get(n)) else {
                return Err(format!("step {}: `$step` {n} must be an earlier step", step + 1));
            };
            let field = match o.get("field") {
                None => "id",
                Some(Value::String(f)) => f.as_str(),
                Some(_) => return Err(format!("step {}: `field` must be a string", step + 1)),
            };
            result.get(field).cloned().ok_or_else(|| format!("step {}: step {n}'s result has no `{field}`", step + 1))
        }
        Value::Object(o) => {
            let mut out = serde_json::Map::with_capacity(o.len());
            for (k, x) in o {
                out.insert(k.clone(), resolve_step_refs(x, results, step, depth + 1)?);
            }
            Ok(Value::Object(out))
        }
        Value::Array(a) => a.iter().map(|x| resolve_step_refs(x, results, step, depth + 1)).collect::<Result<Vec<_>, _>>().map(Value::Array),
        _ => Ok(v.clone()),
    }
}

/// Run a judged run: every step through `step` (the registry, for the interface; the engine alone, for a headless session), the
/// whole of it as ONE undo step named by the label, or none of it. `session` reaches the engine session inside `c`.
pub fn run_batch<C>(
    c: &mut C,
    session: impl Fn(&mut C) -> &mut Session,
    mut step: impl FnMut(&mut C, &str, &Value) -> Result<Value, String>,
    params: &Value,
) -> Result<Value, String> {
    let run = judge(params)?;
    if session(c).active().is_none() {
        return Err("no-document: no drawing is open in this window".into());
    }
    let depth = session(c).active().map_or(0, |d| d.history.undo.len());
    session(c).begin_named_undo_group(Some(run.label.clone()));
    let mut results: Vec<Value> = Vec::with_capacity(run.steps.len());
    let mut failed: Option<String> = None;
    for (i, (name, args)) in run.steps.iter().enumerate() {
        let outcome = resolve_step_refs(args, &results, i, 0).and_then(|args| step(c, name, &args));
        match outcome {
            Ok(result) => results.push(result),
            Err(e) => {
                failed = Some(format!("step-failed: step {} ({name}): {e} -- nothing was changed", i + 1));
                break;
            }
        }
    }
    // A failed run is cancelled: its edits are undone and the journal forgets them.
    session(c).end_undo_group(failed.is_some());
    if let Some(e) = failed {
        return Err(e);
    }
    let steps: Vec<Value> = run.steps.iter().zip(results).map(|((name, _), result)| json!({ "name": name, "result": result })).collect();
    let undo_step = session(c).active().is_some_and(|d| d.history.undo.len() > depth);
    Ok(json!({ "steps": steps, "undoStep": undo_step }))
}

impl Session {
    /// One step of a run: the window's own commands here, every other id through the registry.
    pub fn run_host_step(&mut self, name: &str, args: &Value) -> Result<Value, String> {
        match name {
            "inspect" | "render" => self.run_argument_step(name, args),
            other => self.execute(other, args).map_err(|e| e.to_string()),
        }
    }

    /// The window's own commands (`ARGUMENT_COMMANDS`).
    pub fn run_argument_step(&mut self, name: &str, args: &Value) -> Result<Value, String> {
        match name {
            "inspect" => {
                let depth = args.get("depth").and_then(Value::as_u64).map_or(4, |d| d.min(MAX_INSPECT_DEPTH));
                self.execute("document.inspect", &json!({ "depth": depth })).map_err(|e| e.to_string())
            }
            "render" => self.render_drawing(args),
            other => Err(format!("unknown window command '{other}'")),
        }
    }

    /// One `command.batch` on this session alone (a headless session, the node build the assistant tests drive): all of it as ONE
    /// undo step, or none of it.
    pub fn run_command_batch(&mut self, params: &Value) -> Result<Value, String> {
        run_batch(self, |s| s, |s, name, args| s.run_host_step(name, args), params)
    }

    /// The active drawing encoded as the file `name` names (its extension), the way a Save would write it: the bytes the host
    /// stores as the next version.
    pub fn export_active(&mut self, name: &str) -> Result<Vec<u8>, String> {
        let format = crate::cmd::fileio::writable_format(None, Some(name))?;
        let r = self.execute("document.serialize", &json!({ "format": format.id })).map_err(|e| e.to_string())?;
        if let Some(b64) = r.get("dataBase64").and_then(Value::as_str) {
            return vectorcraft_format::base64_decode(b64).ok_or_else(|| "the encoder returned unreadable data".to_string());
        }
        r.get("text").and_then(Value::as_str).map(|t| t.as_bytes().to_vec()).ok_or_else(|| "the encoder returned no data".to_string())
    }

    fn render_drawing(&mut self, args: &Value) -> Result<Value, String> {
        let doc = self.execute("document.inspect", &json!({ "depth": 0 })).map_err(|e| e.to_string())?;
        let index = args.get("artboard").and_then(Value::as_u64).unwrap_or(0);
        let board = doc
            .get("artboards")
            .and_then(Value::as_array)
            .and_then(|a| a.get(usize::try_from(index).ok()?))
            .ok_or_else(|| format!("no artboard {index}"))?;
        let (w, h) = (board.get("width").and_then(Value::as_f64).unwrap_or(0.0), board.get("height").and_then(Value::as_f64).unwrap_or(0.0));
        if !(w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0) {
            return Err("nothing to render: the artboard has no size".into());
        }
        let scale = (MAX_RENDER_SIDE / w.max(h)).clamp(0.01, 1.0);
        let r = self.execute("document.serialize", &json!({ "format": "png", "scale": scale, "artboard": index })).map_err(|e| e.to_string())?;
        let png = r.get("dataBase64").and_then(Value::as_str).ok_or("the renderer returned no image")?;
        let bytes = vectorcraft_format::base64_decode(png).ok_or("the renderer returned unreadable data")?;
        Ok(json!({ "mime": "image/png", "artboard": index, "bytes": bytes.len(), "png": png }))
    }
}

#[cfg(test)]
mod tests;
