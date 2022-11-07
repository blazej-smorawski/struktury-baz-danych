#include <iostream>
#include <fstream>
#include <optional>
#include <vector>

class blockdevice
{
private:
    std::string filename;
    std::fstream file;
    unsigned blocksize;

public:
    blockdevice(std::string filename, unsigned blocksize);
    ~blockdevice();
    std::optional<std::vector<char> &> read(std::vector<char> &buf, unsigned pos);
};
