function extract()
    local file = io.open("brainworms.txt", "r")
    local content = file:read "*a":gsub("[\f\u{25CC}]+", "")
    file:close()

    local n = 0
    local out = ""
    local prev = ""
    local cur = ""
    local def = false
    local name_cur = ""
    local name_prev = ""

    print "running"
    for l in content:gmatch "[^\n]+" do
    if l:len() > 1 then
        l = l:gsub("\n", "")
        print(n, l)
        prev = cur
        name_prev = name_cur
        local name, tick = l:match "([%w%s%p]+)%s(%-)%s"
        if tick == "-" then
        def = true
        name_cur = name
        cur = l
        else
        cur = cur .. " " .. l
        end

        if name_prev ~= name_cur then
        n = n + 1
        out = out .. prev .. "\n"
        end

        print("PREV", name_prev, prev)
        print("CUR" , name_cur, cur)
    end
    end
    print("end")
    print("total = ", n)
    local outfile = io.open("out.txt", "w")
    outfile:write(out)
    outfile:close()
end

function parse()
    local file = io.open("out.txt", "r")
    local content = file:read "*a"
    file:close()

    local out="id;name;readable_name;desc;added_by;lang\n"

    local id = 0

    for l in content:gmatch "[^\n]+" do
        local name, def = l:match "([%w%s%p]+)%s%-%s(.+)"
        for n in name:gmatch "[^/\\]+" do
            n = n:match "^%s*(.-)%s*$"
            id = id + 1
            local res = id .. ";" .. n:lower() .. ";" .. n .. ";" .. def:gsub("^%l", string.upper) .. ";4ch /tttt/ Dictionary (/source);en\n"
            print(res)
            out = out .. res
        end
    end

    local outfile = io.open("out.csv", "w")
    outfile:write(out)
    outfile:close()
    print("Parsed " .. id .. " entries.")
end

io.write("Select mode (E - extract, P - parse) > ")
local mode = io.read()

if mode:sub(1,1):lower() == "e" then
    extract()
elseif mode:sub(1,1):lower() == "p" then
    parse()
end
