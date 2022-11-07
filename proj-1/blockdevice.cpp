#include "blockdevice.hpp"
#include <iterator>
#include <algorithm>

blockdevice::blockdevice(std::string filename, unsigned blocksize) : filename(filename),
                                                                     file(filename, std::fstream::binary),
                                                                     blocksize(blocksize)
{
}

blockdevice::~blockdevice()
{
    file.close();
}

std::optional<std::vector<char> &> blockdevice::read(std::vector<char> &buf, unsigned pos)
{
    std::istream_iterator<char> start(this->file);
}