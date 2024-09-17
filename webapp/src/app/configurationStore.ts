import { CoreSocketFactory } from "./models/communication/CoreSocketFactory";
import { Subject } from 'rxjs';

export class ConfigurationStore {
    public static nodePositions: { [key: string]: { x: number, y: number } } = {};
    public static networkNodePositions: { [key: string]: number } = {};
    public static stepTime: number = 1000;

    
  public static configurationLoaded = new Subject<string>();

    static saveConfiguration() {
      let socket = CoreSocketFactory.create();

      //create json from nodepositions and networkNodePositions and stepTime
      let configuration = {
        nodePositions: ConfigurationStore.nodePositions,
        networkNodePositions: ConfigurationStore.networkNodePositions,
        stepTime: ConfigurationStore.stepTime
      };

      let json = JSON.stringify(configuration);

      socket.call('store', ["webapp",json]);

      }

    static loadConfiguration() {
      let socket = CoreSocketFactory.create();

      socket.call('load', ["webapp"]).then((result: string) => {
        let configuration = JSON.parse(result);
        ConfigurationStore.nodePositions = configuration.nodePositions;
        ConfigurationStore.networkNodePositions = configuration.networkNodePositions;
        ConfigurationStore.stepTime = configuration.stepTime;

        this.configurationLoaded.next("loaded");
      });


    }
}