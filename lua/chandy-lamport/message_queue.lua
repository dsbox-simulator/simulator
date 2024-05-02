local dsbox = require("dsbox")

local exports = {}

local queue = {}
function exports.recv(timeout)
    if #queue > 0 then
        return table.remove(queue, 1)
    else
        return dsbox.recv(timeout)
    end
end

function exports.recv_iter()
    return exports.recv
end

function exports.recv_filter(filter)
    for i, message in ipairs(queue) do
        if filter(message) then
            return table.remove(queue, i)
        end
    end
    for message in dsbox.recv_iter() do
        if filter(message) then
            return message
        else
            queue[#queue + 1] = message
        end
    end
end

return exports