
export interface LogMessage {
    type: string;
    text: string;
    marker: Marker;
}

export interface Marker {
    label: string;
    color: string;
}

