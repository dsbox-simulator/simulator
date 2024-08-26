
export class ConfigurationStore {
    public static nodePositions: { [key: string]: { x: number, y: number } } = {};
    public static networkNodePositions: { [key: string]: number } = {};

    public static addNetworkNodePosition(nodeId: string, position: number) {
        ConfigurationStore.networkNodePositions[nodeId] = position;
    }

    public static getNetworkNodePosition(nodeId: string): number {
        return ConfigurationStore.networkNodePositions[nodeId];
    }
}