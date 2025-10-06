import {LogMarkerColor} from "./api/types";

export function cssColor(labelColor: LogMarkerColor):string {
    switch (labelColor) {
        case "Black":
            return "#000000";
        case "Red":
            return "#c00000";
        case "Green":
            return "#00c000";
        case "Yellow":
            return "#c0c000";
        case "Blue":
            return "#0000c0";
        case "Magenta":
            return "#c000c0";
        case "Cyan":
            return "#00c0c0";
        case "White":
            return "#c0c0c0";
        case "BrightBlack":
            return "#808080";
        case "BrightRed":
            return "#ff0000";
        case "BrightGreen":
            return "#00ff00";
        case "BrightYellow":
            return "#ffff00";
        case "BrightBlue":
            return "#0000ff";
        case "BrightMagenta":
            return "#ff00ff";
        case "BrightCyan":
            return "#00ffff";
        case "BrightWhite":
            return "#ffffff";
    }
}