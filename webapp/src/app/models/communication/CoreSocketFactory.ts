import CoreSocket from "./CoreSocket";

export class CoreSocketFactory {

    static instance: CoreSocket;

    public static create(): CoreSocket {
        if(this.instance === null) {
            this.instance =  new CoreSocket();
        }

        return this.instance;
    }
}