import {JsonProperty, Serializable} from "ts-jackson";

@Serializable()
export default class Timestamp {
    @JsonProperty()
    logical!: number;
    @JsonProperty()
    physical!: Date;
}