import { CoreSocketFactory } from "./models/communication/CoreSocketFactory";

export class ConfigurationStore {
    public static nodePositions: { [key: string]: { x: number, y: number } } = {};
    public static networkNodePositions: { [key: string]: number } = {};
    public static stepTime: number = 1000;


    static saveConfiguration() {
      let socket = CoreSocketFactory.create();

      //create json from nodepositions and networkNodePositions and stepTime
      let configuration = {
        nodePositions: ConfigurationStore.nodePositions,
        networkNodePositions: ConfigurationStore.networkNodePositions,
        stepTime: ConfigurationStore.stepTime
      };

      let json = JSON.stringify(configuration);
      console.log("Saving configuration:", json);

      socket.call('store', ["webapp",json]);

      }

    static loadConfiguration() {
      console.log("Loading configuration");
      let socket = CoreSocketFactory.create();
      console.log("Loading configuration 2");

      socket.call('load', ["webapp"]).then((result: string) => {
        let configuration = JSON.parse(result);
        console.log("Loaded configuration:", configuration);
        ConfigurationStore.nodePositions = configuration.nodePositions;
        ConfigurationStore.networkNodePositions = configuration.networkNodePositions;
        ConfigurationStore.stepTime = configuration.stepTime;


        console.log("Easyfilterforme Loaded configuration:", ConfigurationStore.networkNodePositions, ConfigurationStore.nodePositions,  ConfigurationStore.stepTime);

      });


    }
}