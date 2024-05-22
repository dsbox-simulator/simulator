export default class Message {
    src!: string;
    dest!: string;
    body!: { type: string; [key: string]: any; }

    static deserialize(jsonMessage: any): Message {
        return {
            src: jsonMessage.src,
            dest: jsonMessage.dest,
            body: jsonMessage.body,
        }
    }
}