import {LogMarkerColor} from "./api/types";

// color scheme taken from https://github.com/Gogh-Co/Gogh, using the "Nord Light" theme.
export function cssColor(labelColor: LogMarkerColor):string {
    switch (labelColor) {
        case "Black":
            return "#003B4E";
        case "Red":
            return "#E64569";
        case "Green":
            return "#069F5F";
        case "Yellow":
            return "#DAB752";
        case "Blue":
            return "#439ECF";
        case "Magenta":
            return "#D961DC";
        case "Cyan":
            return "#00B1BE";
        case "White":
            return "#B3B3B3";
        case "BrightBlack":
            return "#3E89A1";
        case "BrightRed":
            return "#E4859A";
        case "BrightGreen":
            return "#A2CCA1";
        case "BrightYellow":
            return "#E1E387";
        case "BrightBlue":
            return "#6FBBE2";
        case "BrightMagenta":
            return "#E586E7";
        case "BrightCyan":
            return "#96DCDA";
        case "BrightWhite":
            return "#DEDEDE";
    }
}