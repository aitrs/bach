import array
import os
import sys
from random import randrange 


def mkfile(path):
    f = open(path, 'wb')
    size = randrange(10, 104857)
    contents = array.array('B')

    for i in range(size):
        contents.append(randrange(0, 255))

    f.write(contents)
    f.close()


def mkrandname():
    return '{}'.format(randrange(10000000, 99999999))


def create_tree(maxdepth, it, path):
    if maxdepth > 0:
        for i in range(it):
            typ = randrange(10)
            npath = '{}/{}'.format(path, mkrandname())
            if typ > 5:
                mkfile(npath)
            else:
                os.mkdir(npath)
                create_tree(maxdepth - 1, it, npath)


if __name__ == '__main__':
    maxdepth = int(sys.argv[1])
    it = int(sys.argv[2])

    create_tree(maxdepth, it, sys.argv[3])
