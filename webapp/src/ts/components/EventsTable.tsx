import React from "react";
import Event from "../classes/Event";

const EventsTable: React.FC<{ events: Event[] }> = ({events}) => {
    function removeType(key: string, value: any) {
        if (key == "type") {
            return undefined;
        } else {
            return value;
        }
    }

    return <table id="events" className="table table-sm">
        <thead>
        <tr>
            <th>logical</th>
            <th>physical</th>
            <th>type</th>
            <th>data</th>
        </tr>
        </thead>
        <tbody>
        {events.map(event =>
            <tr key={event.timestamp.logical}>
                <td>{event.timestamp.logical}</td>
                <td>{event.timestamp.physical.toISOString()}</td>
                <td>{event.data.type}</td>
                <td>{JSON.stringify(event.data, removeType)}</td>
            </tr>)}
        </tbody>
    </table>;
}

export default EventsTable;